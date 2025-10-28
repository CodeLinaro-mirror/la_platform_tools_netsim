// Copyright 2023-2025 The Android Open Source Project

use bluetooth::server::Server;
use bytes::Bytes;
use futures::{
    sink::Sink,
    stream::Stream,
    task::{Context, Poll},
    Future,
};
use netsim_api::chips::{BluetoothMode, BluetoothParams, ChipClient, ChipConfig, NetworkParams};
use netsim_proto::configuration::Controller as RootcanalController;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Encapsulates the common setup for a test environment.
pub struct TestFixture {
    pub client: ChipClient,
    pub _server_task: JoinHandle<()>, // Keep the task handle to ensure the server runs
}

/// Sets up a test environment with a running server and a client.
pub fn setup() -> TestFixture {
    let (server, client) = Server::new();
    let server_task = tokio::spawn(async move {
        server.run().await;
    });
    TestFixture { client, _server_task: server_task }
}

/// A mock Sink that captures packets into an mpsc channel.
struct MockSink {
    tx: mpsc::Sender<Vec<u8>>,
}

impl Sink<Bytes> for MockSink {
    type Error = std::io::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut pinned = std::pin::pin!(self.get_mut().tx.reserve());
        pinned.as_mut().poll(cx).map(|result| {
            result
                .map(|_| ())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::BrokenPipe, e.to_string()))
        })
    }

    fn start_send(self: Pin<&mut Self>, item: Bytes) -> Result<(), Self::Error> {
        self.tx
            .try_send(item.to_vec())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::BrokenPipe, e.to_string()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

/// Creates a mock packet sink and a receiver to check the captured packets.
pub fn mock_sink(
) -> (Pin<Box<dyn Sink<Bytes, Error = std::io::Error> + Send>>, mpsc::Receiver<Vec<u8>>) {
    let (packet_tx, packet_rx) = mpsc::channel(10);
    let sink = Box::pin(MockSink { tx: packet_tx });
    (sink, packet_rx)
}

use tokio_stream::wrappers::ReceiverStream;

/// Creates a mock packet stream and a sender to inject packets into it.
pub fn mock_stream() -> (Box<dyn Stream<Item = Bytes> + Send + Unpin>, mpsc::Sender<Bytes>) {
    let (packet_tx, packet_rx) = mpsc::channel(10);
    let stream = Box::new(ReceiverStream::new(packet_rx));
    (stream, packet_tx)
}

pub fn create_chip_config(mode: BluetoothMode) -> ChipConfig {
    ChipConfig::new(
        "test_chip",
        "test_manufacturer",
        "test_product",
        NetworkParams::Bluetooth(BluetoothParams {
            address: "AB:CD:EF:11:22:33".to_string(),
            bt_properties: RootcanalController::default(),
            mode,
        }),
    )
}
