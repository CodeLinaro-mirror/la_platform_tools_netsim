use std::sync::Arc;

use bytes::Bytes;
use futures::{pin_mut, stream::StreamExt, SinkExt, TryStreamExt};
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
use tracing::warn;

use crate::packet_stream_converter;

#[derive(Clone)]
pub struct PacketStreamerService {
    // Sender to the GrpcTransportListener (or equivalent) to pass new connections
    new_connection_tx: mpsc::Sender<(ChipInfo, String, PacketStream, PacketSink)>,
}

impl PacketStreamerService {
    pub fn new(
        new_connection_tx: mpsc::Sender<(ChipInfo, String, PacketStream, PacketSink)>,
    ) -> Self {
        Self { new_connection_tx }
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

impl PacketStreamer for PacketStreamerService {
    fn stream_packets(
        &mut self,
        ctx: ::grpcio::RpcContext,
        stream: ::grpcio::RequestStream<PacketRequest>,
        sink: ::grpcio::DuplexSink<PacketResponse>,
    ) {
        let new_connection_tx = self.new_connection_tx.clone();
        let peer_addr = ctx.peer();

        ctx.spawn(async move {
            // Do not call Tokio I/O from within this future, it will panic!
            // https://github.com/tikv/grpc-rs/issues/338
            let (chip_info, stream, sink) = match handle_grpc_initial_info(stream, sink).await {
                Ok(info) => info,
                Err(_) => return, // Error already logged and sink failed
            };

            // Send the new connection's channels to the listener's accept loop
            let is_bt = chip_info
                .chip
                .as_ref()
                .map_or(false, |c| c.kind == netsim_model::initial_info::ChipKind::BLUETOOTH);

            let grpc_stream = stream.map_err(|err| grpc_error_to_packet_error(err)).and_then(
                |packet_request| async {
                    packet_stream_converter::packet_request_to_bytes(packet_request)
                },
            );
            let (packet_stream_tx, packet_stream_rx) = mpsc::channel(100);
            let packet_stream = Box::pin(ReceiverStream::new(packet_stream_rx));

            let packet_sink = Box::pin(
                sink.with_flat_map(move |bytes: Bytes| {
                    match packet_stream_converter::bytes_to_packet_response(bytes, is_bt) {
                        Ok(packet_response) => futures::stream::iter(vec![Ok((
                            packet_response,
                            grpcio::WriteFlags::default()
                                .buffer_hint(false)
                                .force_no_compress(true),
                        ))]),
                        Err(err) => {
                            warn!("Error converting bytes to packet response: {err:?}");
                            futures::stream::iter(vec![])
                        }
                    }
                })
                .sink_map_err(grpc_error_to_packet_error),
            );

            if new_connection_tx
                .send((chip_info, peer_addr, packet_stream, packet_sink))
                .await
                .is_err()
            {
                warn!("Failed to send new connection to listener. Dropping connection.");
            }

            // The stream must be driven within gRPC, or tokio won't get a wake.
            // https://github.com/tikv/grpc-rs/issues/338#issuecomment-512845420
            pin_mut!(grpc_stream);
            while let Some(packet_res) = grpc_stream.next().await {
                if packet_stream_tx.send(packet_res).await.is_err() {
                    break;
                }
            }
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
    pub rx: mpsc::Receiver<(ChipInfo, String, PacketStream, PacketSink)>,
    pub local_addr: packet_stream::StreamAddress,
}

#[async_trait::async_trait]
impl packet_stream::transport::traits::TransportListener for ChannelTransportListener {
    async fn accept(
        &mut self,
    ) -> packet_stream::error::Result<(PacketStream, PacketSink, ChipInfo, String)> {
        match self.rx.recv().await {
            Some((chip_info, guid, app_stream, app_sink)) => {
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
