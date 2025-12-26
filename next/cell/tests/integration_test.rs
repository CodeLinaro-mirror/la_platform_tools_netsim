// next/cell/tests/integration_test.rs
use bytes::Bytes;
use cell::server::CellServer;
use env_logger;
use futures::{channel::mpsc as fmpsc, future::ready, sink::SinkExt, stream::StreamExt};
use netsim_api::chip_error::ChipError as NetsimChipError;
use netsim_api::chips::{
    CellParams, ChipClient, ChipConfig, ChipDiedParams, ChipId, CreateParams as CreateChipParams,
    NetworkParams, PacketSink, PacketStream,
};

use std::io::Error as IoError;
use std::io::ErrorKind;
use std::pin::Pin;
use tokio::sync::mpsc;

// Helper to create a dummy PacketStream and PacketSink
fn create_dummy_stream_sink(
) -> (PacketStream, PacketSink, fmpsc::Sender<Bytes>, fmpsc::Receiver<Bytes>) {
    let (stream_tx, stream_rx) = fmpsc::channel::<Bytes>(64);
    let (sink_tx, sink_rx) = fmpsc::channel::<Bytes>(64);

    let stream: PacketStream = Box::new(stream_rx);
    let sink: PacketSink = Pin::from(Box::new(
        sink_tx
            .with(|data: Bytes| ready(Ok(data)))
            .sink_map_err(|e: fmpsc::SendError| IoError::new(ErrorKind::Other, e.to_string())),
    ));

    (stream, sink, stream_tx, sink_rx)
}

struct TestHarness {
    client: ChipClient,
    device_server_rx: mpsc::Receiver<ChipDiedParams>,
    server_handle: tokio::task::JoinHandle<()>,
}

async fn setup_test_harness() -> TestHarness {
    let _ = env_logger::try_init();
    let (command_tx, command_rx) = mpsc::channel(100);
    let (device_server_tx, device_server_rx) = mpsc::channel(100);

    let fake_controller = cell::fake_modem_network::FakeModemNetwork::new();
    let server = CellServer::new(device_server_tx, command_rx, fake_controller);
    let server_handle = tokio::spawn(server.run());
    let client = ChipClient::new(command_tx);

    TestHarness { client, device_server_rx, server_handle }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        self.server_handle.abort();
    }
}

fn create_params(chip_id: ChipId, stream: PacketStream, sink: PacketSink) -> CreateChipParams {
    CreateChipParams {
        id: chip_id,
        packet_stream: Some(stream),
        packet_sink: Some(sink),
        config: ChipConfig {
            name: format!("cell-{}", chip_id),
            manufacturer: "Netsim".to_string(),
            product_name: "CellEmulator".to_string(),
            network_params: NetworkParams::Cell(CellParams::default()),
        },
    }
}

// T011: Test for CreateChip message
#[tokio::test]
async fn test_create_chip() {
    let harness = setup_test_harness().await;
    let chip_id = ChipId(1);
    let (stream, sink, _, _) = create_dummy_stream_sink();
    let params = create_params(chip_id, stream, sink);
    let response = harness.client.create(params).await;
    assert!(response.is_ok(), "CreateChip failed: {:?}", response);
}

// T012: Test for DeleteChip message
#[tokio::test]
async fn test_delete_chip() {
    let mut harness = setup_test_harness().await;
    let chip_id = ChipId(2);
    let (stream, sink, _, _) = create_dummy_stream_sink();
    let params = create_params(chip_id, stream, sink);
    harness.client.create(params).await.unwrap();

    let response = harness.client.delete(chip_id).await;
    assert!(response.is_ok(), "DeleteChip failed: {:?}", response);

    match tokio::time::timeout(std::time::Duration::from_secs(1), harness.device_server_rx.recv())
        .await
    {
        Ok(Some(params)) => assert_eq!(params.id, chip_id),
        _ => panic!("Did not receive ChipDied message"),
    }
}

// T013: Test for stream/sink error triggering chip deletion
#[tokio::test]
async fn test_stream_error_triggers_delete() {
    let mut harness = setup_test_harness().await;
    let chip_id = ChipId(3);
    let (stream, sink, mut stream_tx, _) = create_dummy_stream_sink();
    let params = create_params(chip_id, stream, sink);
    harness.client.create(params).await.unwrap();

    // Close the stream sender to simulate an error
    stream_tx.close_channel();

    match tokio::time::timeout(std::time::Duration::from_secs(2), harness.device_server_rx.recv())
        .await
    {
        Ok(Some(params)) => assert_eq!(params.id, chip_id),
        _ => panic!("Did not receive ChipDied message on stream error"),
    }
}

// T020: Test stream to controller passthrough and ECHO response
#[tokio::test]
async fn test_stream_to_controller_echo() {
    let harness = setup_test_harness().await;
    let chip_id = ChipId(4);
    let (stream, sink, mut stream_tx, mut sink_rx) = create_dummy_stream_sink();
    let params = create_params(chip_id, stream, sink);
    harness.client.create(params).await.unwrap();

    let test_data = Bytes::from_static(b"AT+CGMI\r\n");
    stream_tx.send(test_data.clone()).await.unwrap();

    // Expect ECHO response from StubCellularController
    match tokio::time::timeout(std::time::Duration::from_millis(500), sink_rx.next()).await {
        Ok(Some(data)) => {
            assert_eq!(data, Bytes::from(format!("ECHO: {}", String::from_utf8_lossy(&test_data))));
        }
        _ => panic!("Did not receive ECHO response"),
    }
}

// T028: Test GetChip message
#[tokio::test]
async fn test_get_chip() {
    let harness = setup_test_harness().await;
    let chip_id = ChipId(1);
    let (stream, sink, _stream_tx, _sink_rx) = create_dummy_stream_sink(); // Keep _stream_tx and _sink_rx in scope
    let params = create_params(chip_id, stream, sink);
    let create_response = harness.client.create(params).await;
    assert!(create_response.is_ok(), "CreateChip failed: {:?}", create_response);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let get_chip_response = harness.client.read(chip_id).await;
    match get_chip_response {
        Ok(netsim_api::chips::ChipInfo::Cell(cell_info)) => {
            assert_eq!(cell_info.chip_id, chip_id);
            assert_eq!(cell_info.state, "FAKE_ACTIVE");
        }
        Ok(other) => panic!("Expected ChipInfo::Cell, got {:?}", other),
        Err(e) => panic!("GetChip failed for existing chip: {:?}", e),
    }

    // Test non-existent chip
    let bad_chip_id = ChipId(99);
    match harness.client.read(bad_chip_id).await {
        Err(netsim_api::client_error::ClientError::Chip(NetsimChipError::ChipNotFound(id))) => {
            assert_eq!(id, bad_chip_id);
        }
        other => panic!("Expected ChipNotFound error, got {:?}", other),
    }
}
// T021 & T026 are implicitly tested by the echo in test_stream_to_controller_echo
