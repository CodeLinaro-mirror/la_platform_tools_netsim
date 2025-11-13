// tests/test_grpc.rs
use bytes::Bytes;
use futures::stream::{StreamExt, TryStreamExt};
use futures::SinkExt;
use grpcio::{ClientDuplexReceiver, ClientDuplexSender, Error as GrpcError, WriteFlags};
use netsim_api::initial_info::{ChipInfo, ChipKind};
use netsim_proto::packet_streamer::{PacketRequest, PacketResponse};
use packet_stream::error::{PacketStreamError, Result};
use packet_stream::transport::grpc_converters;
use packet_stream::transport::traits::TransportListener;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Bridge to forward packets between a gRPC duplex stream and MPSC channels for the CLIENT side.
async fn run_client_bidi_bridge(
    grpc_rx: ClientDuplexReceiver<PacketResponse>, // gRPC Client receives PacketResponse
    grpc_tx: ClientDuplexSender<PacketRequest>,    // gRPC Client sends PacketRequest
    mut mpsc_rx: mpsc::Receiver<Bytes>,            // Receives from App to send to gRPC
    mpsc_tx: mpsc::Sender<Result<Bytes>>,          // Sends to App from gRPC
) -> Result<()> {
    let mut grpc_tx = grpc_tx
        .with(|req: PacketRequest| {
            Box::pin(async { Ok::<_, GrpcError>((req, WriteFlags::default())) })
        })
        .sink_map_err(PacketStreamError::from);
    let mut grpc_rx = grpc_rx.map_err(PacketStreamError::from);

    loop {
        tokio::select! {
            // gRPC -> MPSC (App)
            grpc_msg = grpc_rx.next() => {
                match grpc_msg {
                    Some(Ok(packet_response)) => {
                        match grpc_converters::packet_response_to_bytes(packet_response) {
                            Ok(bytes) => {
                                if mpsc_tx.send(Ok(bytes)).await.is_err() {
                                    eprintln!("Client Bridge: mpsc_tx.send failed after packet_response_to_bytes");
                                    return Err(PacketStreamError::ConnectionClosed);
                                }
                            }
                            Err(e) => {
                                if mpsc_tx.send(Err(e)).await.is_err() {
                                    eprintln!("Client Bridge: mpsc_tx.send failed after packet_response_to_bytes error");
                                    return Err(PacketStreamError::ConnectionClosed);
                                }
                            }
                        }
                    }
                    Some(Err(e)) => {
                        let _ = mpsc_tx.send(Err(e)).await;
                        return Err(PacketStreamError::ConnectionClosed);
                    }
                    None => return Ok(()), // gRPC stream closed gracefully
                }
            }
            // MPSC (App) -> gRPC
            mpsc_msg = mpsc_rx.recv() => {
                match mpsc_msg {
                    Some(bytes) => {
                        match grpc_converters::bytes_to_packet_request(bytes) {
                            Ok(packet_request) => {
                                if let Err(e) = grpc_tx.send(packet_request).await {
                                    eprintln!("Client Bridge: grpc_tx.send error: {:?}", e);
                                    let _ = mpsc_tx.send(Err(e)).await;
                                    return Err(PacketStreamError::ConnectionClosed);
                                }
                            }
                            Err(e) => {
                                 let _ = mpsc_tx.send(Err(e)).await;
                            }
                        }
                    }
                    None => {
                        let _ = grpc_tx.close().await;
                        return Ok(());
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn test_grpc_transport_echo() {
    // Setup gRPC server
    let listener =
        packet_stream::transport::grpc_adapter::GrpcTransportListener::bind("127.0.0.1", 0)
            .await
            .unwrap();
    let addr = match listener.local_addr().unwrap() {
        packet_stream::StreamAddress::Grpc(addr) => addr,
        _ => panic!("Expected gRPC address"),
    };

    let server_task = tokio::spawn(async move {
        let mut listener = listener;

        let (mut stream, mut sink, chip_info, _guid) =
            listener.accept().await.expect("Server accept failed");
        assert_eq!(chip_info.name, "test_chip");
        while let Some(Ok(packet)) = futures::StreamExt::next(&mut stream).await {
            futures::SinkExt::send(&mut sink, packet).await.expect("Server send failed");
        }
        listener.shutdown().await.expect("Server shutdown failed");
    });

    // Setup gRPC client
    let chip_info = ChipInfo {
        name: "test_chip".to_string(),
        chip: Some(netsim_api::initial_info::Chip {
            kind: ChipKind::BLUETOOTH,
            id: "00".to_string(),
            name: "".to_string(),
            manufacturer: "".to_string(),
            product_name: "".to_string(),
        }),
        device_info: None,
    };
    let (mut app_stream, mut app_sink, client, client_send, client_recv) =
        packet_stream::transport::grpc_adapter::connect("127.0.0.1", addr.port(), chip_info)
            .await
            .unwrap();

    // Create MPSC channels for test communication
    let (mpsc_tx_app_out, mpsc_rx_bridge_in) = mpsc::channel::<Bytes>(100);
    let (mpsc_tx_bridge_out, mpsc_rx_app_in) = mpsc::channel::<Result<Bytes>>(100);

    // Spawn the client bridge task
    tokio::spawn(async move {
        if let Err(e) =
            run_client_bidi_bridge(client_recv, client_send, mpsc_rx_bridge_in, mpsc_tx_bridge_out)
                .await
        {
            eprintln!("Client Bridge task error: {e}");
        }
    });

    // Recreate app_stream and app_sink with the new channels
    app_stream = Box::pin(ReceiverStream::new(mpsc_rx_app_in));
    app_sink = Box::pin(futures::sink::unfold(mpsc_tx_app_out, |tx, item: Bytes| async move {
        tx.send(item).await.map_err(|e| {
            PacketStreamError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                e.to_string(),
            ))
        })?;
        Ok(tx)
    }));

    // Test sending and receiving
    let test_packet = bytes::Bytes::from_static(&[0xaa, 0xbb, 0xcc, 0xdd]);
    futures::SinkExt::send(&mut app_sink, test_packet.clone()).await.unwrap();
    let received = futures::StreamExt::next(&mut app_stream).await.unwrap().unwrap();
    assert_eq!(received, test_packet);

    // Close and shutdown
    drop(app_sink);
    let _ = futures::StreamExt::next(&mut app_stream).await;
    server_task.await.unwrap();
    drop(client); // Drop the client to close the connection
}
