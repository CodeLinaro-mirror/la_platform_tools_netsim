use super::grpc_converters;
use crate::error::{PacketStreamError, Result};
use crate::transport::traits::{PacketSink, PacketStream, TransportListener};
use bytes::Bytes;
use futures::stream::StreamExt;
use futures::SinkExt;
use log::warn;
use netsim_api::initial_info::ChipInfo;
use netsim_proto::packet_streamer::{self, PacketRequest, PacketResponse};
use netsim_proto::packet_streamer_grpc::{create_packet_streamer, PacketStreamer};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

#[derive(Clone)]
pub struct PacketStreamerService {
    // Sender to the GrpcTransportListener to pass new connections
    new_connection_tx:
        mpsc::Sender<(ChipInfo, String, mpsc::Receiver<Result<Bytes>>, mpsc::Sender<Bytes>)>,
}

impl PacketStreamerService {
    pub fn new(
        new_connection_tx: mpsc::Sender<(
            ChipInfo,
            String,
            mpsc::Receiver<Result<Bytes>>,
            mpsc::Sender<Bytes>,
        )>,
    ) -> Self {
        PacketStreamerService { new_connection_tx }
    }
}

async fn handle_grpc_initial_info(
    mut stream: ::grpcio::RequestStream<PacketRequest>,
    sink: ::grpcio::DuplexSink<PacketResponse>,
) -> Result<(ChipInfo, ::grpcio::RequestStream<PacketRequest>, ::grpcio::DuplexSink<PacketResponse>)>
{
    let initial_info_req = match stream.next().await {
        Some(Ok(req)) => req,
        Some(Err(e)) => {
            eprintln!("gRPC stream error on initial info: {e}");
            let _ = sink
                .fail(grpcio::RpcStatus::with_message(
                    grpcio::RpcStatusCode::INVALID_ARGUMENT,
                    format!("Error receiving initial info: {e}"),
                ))
                .await;
            return Err(e.into());
        }
        None => {
            eprintln!("gRPC stream closed before initial info");
            let _ = sink
                .fail(grpcio::RpcStatus::with_message(
                    grpcio::RpcStatusCode::INVALID_ARGUMENT,
                    "Stream closed before initial info".to_string(),
                ))
                .await;
            return Err(PacketStreamError::ConnectionClosed);
        }
    };

    match initial_info_req.request_type {
        Some(packet_streamer::packet_request::Request_type::InitialInfo(info)) => {
            Ok((grpc_converters::proto_to_chip_info(info), stream, sink))
        }
        _ => {
            eprintln!("First message was not InitialInfo");
            let _ = sink
                .fail(grpcio::RpcStatus::with_message(
                    grpcio::RpcStatusCode::INVALID_ARGUMENT,
                    "First message must be InitialInfo".to_string(),
                ))
                .await;
            Err(PacketStreamError::Protocol(crate::error::ProtocolError::InvalidFormat(
                "First message must be InitialInfo".to_string(),
            )))
        }
    }
}

async fn pump_grpc_messages(
    mut stream: ::grpcio::RequestStream<PacketRequest>,
    mut sink: ::grpcio::DuplexSink<PacketResponse>,
    grpc_to_app_tx: mpsc::Sender<Result<Bytes>>,
    mut app_to_grpc_rx: mpsc::Receiver<Bytes>,
    is_bt: bool,
) -> Result<()> {
    loop {
        tokio::select! {
            // Received from gRPC client
            grpc_msg = stream.next() => {
                match grpc_msg {
                    Some(Ok(packet_request)) => {
                        let bytes_result = grpc_converters::packet_request_to_bytes(packet_request);
                        if grpc_to_app_tx.send(bytes_result).await.is_err() {
                            break; // App side closed
                        }
                    }
                    Some(Err(e)) => {
                        let _ = grpc_to_app_tx.send(Err(e.into())).await;
                        break;
                    }
                    None => break, // gRPC client closed
                }
            }
            // Received from App
            mpsc_msg = app_to_grpc_rx.recv() => {
                match mpsc_msg {
                    Some(bytes) => {
                        match grpc_converters::bytes_to_packet_response(bytes, is_bt) {
                            Ok(packet_response) => {
                                if sink.send((packet_response, grpcio::WriteFlags::default())).await.is_err() {
                                    break; // gRPC client closed
                                }
                            }
                            Err(e) => {
                                warn!("Error converting bytes to packet response: {:?}", e);
                                // Consider propagating this error to the app
                            }
                        }
                    }
                    None => break, // App side closed
                }
            }
        }
    }
    let _ = sink.close().await;
    Ok(())
}

impl PacketStreamer for PacketStreamerService {
    fn stream_packets(
        &mut self,
        _ctx: ::grpcio::RpcContext,
        stream: ::grpcio::RequestStream<PacketRequest>,
        sink: ::grpcio::DuplexSink<PacketResponse>,
    ) {
        let new_connection_tx = self.new_connection_tx.clone();
        let peer_addr = _ctx.peer();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let (chip_info, stream, sink) = match handle_grpc_initial_info(stream, sink).await {
                    Ok(info) => info,
                    Err(_) => return, // Error already logged and sink failed
                };

                // Create channels for this connection
                let (grpc_to_app_tx, grpc_to_app_rx) = mpsc::channel::<Result<Bytes>>(100);
                let (app_to_grpc_tx, app_to_grpc_rx) = mpsc::channel::<Bytes>(100);

                // Send the new connection's channels to the listener's accept loop
                let is_bt = chip_info
                    .chip
                    .as_ref()
                    .map_or(false, |c| c.kind == netsim_api::initial_info::ChipKind::BLUETOOTH);
                if new_connection_tx
                    .send((chip_info, peer_addr, grpc_to_app_rx, app_to_grpc_tx))
                    .await
                    .is_err()
                {
                    warn!("Failed to send new connection to listener. Dropping connection.");
                    return;
                }

                if let Err(e) =
                    pump_grpc_messages(stream, sink, grpc_to_app_tx, app_to_grpc_rx, is_bt).await
                {
                    warn!("gRPC message pump error: {:?}", e);
                }
            });
        });
    }
}
pub struct GrpcTransportListener {
    server: grpcio::Server,
    // Receiver for new connections from PacketStreamerService
    new_connection_rx:
        mpsc::Receiver<(ChipInfo, String, mpsc::Receiver<Result<Bytes>>, mpsc::Sender<Bytes>)>,
    local_addr: std::net::SocketAddr,
}

impl GrpcTransportListener {
    pub async fn bind(addr: &str, port: u16) -> Result<Self> {
        let (new_connection_tx, new_connection_rx) = mpsc::channel(100);
        let service = create_packet_streamer(PacketStreamerService::new(new_connection_tx));
        let env = Arc::new(grpcio::Environment::new(2));
        let mut server = grpcio::ServerBuilder::new(env).register_service(service).build()?;

        let listen_addr = format!("{}:{}", addr, port);
        let bound_port = server
            .add_listening_port(&listen_addr, grpcio::ServerCredentials::insecure())
            .map_err(|e| {
                PacketStreamError::Socket(crate::error::SocketError::BindFailed(format!(
                    "{} - {}",
                    listen_addr, e
                )))
            })?;

        server.start();

        let local_addr = format!("{}:{}", addr, bound_port).parse()?;

        Ok(GrpcTransportListener { server, new_connection_rx, local_addr })
    }
}

#[async_trait::async_trait]
impl TransportListener for GrpcTransportListener {
    async fn accept(&mut self) -> Result<(PacketStream, PacketSink, ChipInfo, String)> {
        match self.new_connection_rx.recv().await {
            Some((chip_info, guid, grpc_to_app_rx, app_to_grpc_tx)) => {
                // Create PacketStream and PacketSink for the application
                let app_stream: PacketStream = Box::pin(ReceiverStream::new(grpc_to_app_rx));
                let app_sink: PacketSink =
                    Box::pin(futures::sink::unfold(app_to_grpc_tx, |tx, item: Bytes| async move {
                        tx.send(item).await.map_err(|e| {
                            PacketStreamError::Io(std::io::Error::new(
                                std::io::ErrorKind::BrokenPipe,
                                e.to_string(),
                            ))
                        })?;
                        Ok(tx)
                    }));

                Ok((app_stream, app_sink, chip_info, guid))
            }
            None => Err(PacketStreamError::ConnectionClosed),
        }
    }

    fn local_addr(&self) -> Result<crate::types::StreamAddress> {
        Ok(crate::types::StreamAddress::Grpc(self.local_addr))
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.server.shutdown().await.map_err(PacketStreamError::from)
    }
}

pub async fn connect(
    addr: &str,
    port: u16,
    chip_info: ChipInfo,
) -> Result<(
    PacketStream,
    PacketSink,
    netsim_proto::packet_streamer_grpc::PacketStreamerClient,
    grpcio::ClientDuplexSender<PacketRequest>,
    grpcio::ClientDuplexReceiver<PacketResponse>,
)> {
    let env = Arc::new(grpcio::Environment::new(2));
    let channel = grpcio::ChannelBuilder::new(env).connect(&format!("{}:{}", addr, port));
    let client = netsim_proto::packet_streamer_grpc::PacketStreamerClient::new(channel);

    let (mut client_send, client_recv) = client.stream_packets()?;

    // Send InitialInfo
    let mut initial_req = PacketRequest::new();
    initial_req.set_initial_info(grpc_converters::chip_info_to_proto(chip_info));
    client_send.send((initial_req, grpcio::WriteFlags::default())).await?;

    // Create MPSC channels for application communication
    let (mpsc_tx_app_out, _mpsc_rx_bridge_in) = mpsc::channel::<Bytes>(100);
    let (_mpsc_tx_bridge_out, mpsc_rx_app_in) = mpsc::channel::<Result<Bytes>>(100);

    // This bridge is now part of the test, not spawned here.

    // Create PacketStream and PacketSink for the application
    let app_stream: PacketStream = Box::pin(ReceiverStream::new(mpsc_rx_app_in));
    let app_sink: PacketSink =
        Box::pin(futures::sink::unfold(mpsc_tx_app_out, |tx, item: Bytes| async move {
            tx.send(item).await.map_err(|e| {
                PacketStreamError::Io(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    e.to_string(),
                ))
            })?;
            Ok(tx)
        }));

    Ok((app_stream, app_sink, client, client_send, client_recv))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::grpc_converters::*;
    use bytes::{BufMut, Bytes, BytesMut};
    use netsim_proto::hci_packet::HCIPacket;
    use netsim_proto::startup::ChipInfo;
    use protobuf::Message;

    #[test]
    fn test_hci_request_conversion() {
        let mut hci = HCIPacket::new();
        hci.packet = vec![0x01, 0x02, 0x03, 0x04];
        let mut req = PacketRequest::new();
        req.set_hci_packet(hci.clone());

        let bytes = packet_request_to_bytes(req).unwrap();
        assert_eq!(bytes.len(), hci.packet.len());
        assert_eq!(bytes, hci.packet.as_slice());

        // Test bytes_to_packet_request with the type prefix
        let mut bytes_with_prefix = BytesMut::new();
        bytes_with_prefix.put_u8(HCI_PACKET_TYPE);
        bytes_with_prefix.put_slice(&hci.write_to_bytes().unwrap());
        let req2 = bytes_to_packet_request(bytes_with_prefix.freeze()).unwrap();
        assert!(req2.has_hci_packet());
        assert_eq!(req2.hci_packet(), &hci);
    }

    #[test]
    fn test_hci_response_conversion() {
        let packet_data = vec![0x05, 0x06, 0x07, 0x08];
        let mut hci = HCIPacket::new();
        hci.packet = packet_data.clone();

        // Test packet_response_to_bytes
        let mut res = PacketResponse::new();
        res.set_hci_packet(hci.clone());
        let bytes_with_prefix = packet_response_to_bytes(res).unwrap();
        assert_eq!(bytes_with_prefix[0], HCI_PACKET_TYPE);
        assert_eq!(bytes_with_prefix.len(), hci.write_to_bytes().unwrap().len() + 1);

        // Test bytes_to_packet_response with is_bt = true
        let res2 = bytes_to_packet_response(Bytes::from(packet_data.clone()), true).unwrap();
        assert!(res2.has_hci_packet());
        let expected_hci = HCIPacket {
            packet_type: netsim_proto::hci_packet::hcipacket::PacketType::EVENT.into(),
            packet: packet_data.clone(),
            ..Default::default()
        };
        assert_eq!(res2.hci_packet(), &expected_hci);

        // Test bytes_to_packet_response with is_bt = false
        let res3 = bytes_to_packet_response(Bytes::from(packet_data.clone()), false).unwrap();
        assert!(res3.has_packet());
        assert_eq!(res3.packet(), packet_data.as_slice());
    }

    #[test]
    fn test_raw_packet_request() {
        let raw = Bytes::from_static(&[0xAA, 0xBB, 0xCC]);
        let mut req = PacketRequest::new();
        req.set_packet(raw.to_vec());
        let bytes = packet_request_to_bytes(req).unwrap();
        assert_eq!(bytes, raw); // No type byte prepended

        let req2 = bytes_to_packet_request(bytes).unwrap();
        assert!(req2.has_packet());
        assert_eq!(req2.packet(), raw.as_ref());
        assert!(!req2.has_hci_packet());
    }

    #[test]
    fn test_raw_packet_response() {
        let raw = Bytes::from_static(&[0xDD, 0xEE, 0xFF]);
        let mut res = PacketResponse::new();
        res.set_packet(raw.to_vec());
        let bytes = packet_response_to_bytes(res).unwrap();
        assert_eq!(bytes, raw); // No type byte prepended

        let res2 = bytes_to_packet_response(bytes, false).unwrap();
        assert!(res2.has_packet());
        assert_eq!(res2.packet(), raw.as_ref());
        assert!(!res2.has_hci_packet());
    }

    #[test]
    fn test_packet_request_to_bytes_error() {
        let mut req = PacketRequest::new();
        req.set_initial_info(ChipInfo::new());
        let result = packet_request_to_bytes(req);
        assert!(result.is_err());
    }

    #[test]
    fn test_packet_response_to_bytes_error() {
        let mut res = PacketResponse::new();
        res.set_error("Test Error".to_string());
        let result = packet_response_to_bytes(res);
        assert!(result.is_err());
    }
}
