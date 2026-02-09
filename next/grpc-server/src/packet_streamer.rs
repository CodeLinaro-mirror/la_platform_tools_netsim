use std::sync::Arc;

use bytes::Bytes;
use futures::{stream::StreamExt, SinkExt};
use log::warn;
use netsim_model::initial_info::ChipInfo;
use netsim_proto::{
    packet_streamer::{self, PacketRequest, PacketResponse},
    packet_streamer_grpc::{self, PacketStreamer},
};
use packet_stream::{
    error::{PacketStreamError, Result},
    transport::traits::{PacketSink, PacketStream},
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::packet_stream_converter;

#[derive(Clone)]
pub struct PacketStreamerService {
    // Sender to the GrpcTransportListener (or equivalent) to pass new connections
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

fn grpc_error_to_packet_error(e: grpcio::Error) -> PacketStreamError {
    PacketStreamError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
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
            return Err(grpc_error_to_packet_error(e));
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
            Ok((packet_stream_converter::proto_to_chip_info(info), stream, sink))
        }
        _ => {
            eprintln!("First message was not InitialInfo");
            let _ = sink
                .fail(grpcio::RpcStatus::with_message(
                    grpcio::RpcStatusCode::INVALID_ARGUMENT,
                    "First message must be InitialInfo".to_string(),
                ))
                .await;
            Err(PacketStreamError::Protocol(packet_stream::error::ProtocolError::InvalidFormat(
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
                        let bytes_result = packet_stream_converter::packet_request_to_bytes(packet_request);
                        if grpc_to_app_tx.send(bytes_result).await.is_err() {
                            break; // App side closed
                        }
                    }
                    Some(Err(e)) => {
                        let _ = grpc_to_app_tx.send(Err(grpc_error_to_packet_error(e))).await;
                        break;
                    }
                    None => break, // gRPC client closed
                }
            }
            // Received from App
            mpsc_msg = app_to_grpc_rx.recv() => {
                match mpsc_msg {
                    Some(bytes) => {
                        match packet_stream_converter::bytes_to_packet_response(bytes, is_bt) {
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
                    .map_or(false, |c| c.kind == netsim_model::initial_info::ChipKind::BLUETOOTH);
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

pub async fn connect(
    addr: &str,
    port: u16,
    chip_info: ChipInfo,
) -> Result<(
    PacketStream,
    PacketSink,
    packet_streamer_grpc::PacketStreamerClient,
    grpcio::ClientDuplexSender<PacketRequest>,
    grpcio::ClientDuplexReceiver<PacketResponse>,
)> {
    let env = Arc::new(grpcio::Environment::new(2));
    let channel = grpcio::ChannelBuilder::new(env).connect(&format!("{}:{}", addr, port));
    let client = packet_streamer_grpc::PacketStreamerClient::new(channel);

    let (mut client_send, client_recv) =
        client.stream_packets().map_err(grpc_error_to_packet_error)?;

    // Send InitialInfo
    let mut initial_req = PacketRequest::new();
    initial_req.set_initial_info(packet_stream_converter::chip_info_to_proto(chip_info));
    client_send
        .send((initial_req, grpcio::WriteFlags::default()))
        .await
        .map_err(grpc_error_to_packet_error)?;

    // Create MPSC channels for application communication
    let (mpsc_tx_app_out, _mpsc_rx_bridge_in) = mpsc::channel::<Bytes>(100);
    let (_mpsc_tx_bridge_out, mpsc_rx_app_in) = mpsc::channel::<Result<Bytes>>(100);

    // This bridge is now part of the test, not spawned here.
    // Note: The caller is responsible for bridging mpsc_rx_bridge_in and
    // mpsc_tx_bridge_out to the client_send/client_recv if they want a full
    // loop, or just using client directly.

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

pub struct ChannelTransportListener {
    pub rx: mpsc::Receiver<(ChipInfo, String, mpsc::Receiver<Result<Bytes>>, mpsc::Sender<Bytes>)>,
    pub local_addr: packet_stream::StreamAddress,
}

#[async_trait::async_trait]
impl packet_stream::transport::traits::TransportListener for ChannelTransportListener {
    async fn accept(
        &mut self,
    ) -> packet_stream::error::Result<(PacketStream, PacketSink, ChipInfo, String)> {
        match self.rx.recv().await {
            Some((chip_info, guid, grpc_to_app_rx, app_to_grpc_tx)) => {
                let app_stream: PacketStream = Box::pin(ReceiverStream::new(grpc_to_app_rx));
                let app_sink: PacketSink = Box::pin(futures::sink::unfold(
                    app_to_grpc_tx,
                    |tx, item: bytes::Bytes| async move {
                        tx.send(item).await.map_err(|e| {
                            packet_stream::error::PacketStreamError::Io(std::io::Error::new(
                                std::io::ErrorKind::BrokenPipe,
                                e.to_string(),
                            ))
                        })?;
                        Ok(tx)
                    },
                ));
                Ok((app_stream, app_sink, chip_info, guid))
            }
            None => Err(packet_stream::error::PacketStreamError::ConnectionClosed),
        }
    }
    fn local_addr(&self) -> packet_stream::error::Result<packet_stream::StreamAddress> {
        Ok(self.local_addr.clone())
    }
    async fn shutdown(&mut self) -> packet_stream::error::Result<()> {
        Ok(())
    }
}
