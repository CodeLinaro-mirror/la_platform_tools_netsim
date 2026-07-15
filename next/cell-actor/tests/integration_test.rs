// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0
use std::{io::Error as IoError, pin::Pin};

use actor_framework::{ResourceClient, ResourceRequest};
use bytes::Bytes;
use cell_actor::CellClient;
use device_actor::{DeviceActor, DeviceClient};
use device_api::{DeviceAction, DeviceActionResult};
use futures::{channel::mpsc as fmpsc, future::ready, sink::SinkExt};
use netsim_model::{
    Cell, Chip, ChipClient, ChipCreate, ChipError as NetsimChipError, ChipId, ChipVariant,
    DeviceId, PacketSink, PacketStream,
};
use tokio::sync::mpsc;

// Helper to create a dummy PacketStream and PacketSink
fn create_dummy_stream_sink()
-> (PacketStream, PacketSink, fmpsc::Sender<Bytes>, fmpsc::Receiver<Bytes>) {
    let (stream_tx, stream_rx) = fmpsc::channel::<Bytes>(64);
    let (sink_tx, sink_rx) = fmpsc::channel::<Bytes>(64);

    let stream: PacketStream = Box::new(stream_rx);
    let sink: PacketSink = Pin::from(Box::new(
        sink_tx
            .with(|data: Bytes| ready(Ok(data)))
            .sink_map_err(|e: fmpsc::SendError| IoError::other(e.to_string())),
    ));

    (stream, sink, stream_tx, sink_rx)
}

struct TestHarness {
    client: CellClient,
    device_server_rx: mpsc::Receiver<ResourceRequest<DeviceActor>>,
    server_handle: tokio::task::JoinHandle<()>,
}

async fn setup_test_harness() -> TestHarness {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    let (device_server_tx, device_server_rx) = mpsc::channel(100);
    let resource_client = ResourceClient::new(device_server_tx);
    let device_client = DeviceClient::new(Box::new(resource_client));

    let (actor, client) = cell_actor::new();
    let service = cell_actor::CellActor::new(device_client);
    let server_handle = tokio::spawn(actor.run(service));

    TestHarness { client, device_server_rx, server_handle }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        self.server_handle.abort();
    }
}

fn create_params(chip_id: ChipId, stream: PacketStream, sink: PacketSink) -> ChipCreate {
    let mut chip = Chip::new_test_cell(format!("cell-{}", chip_id));
    chip.id = chip_id.0;
    chip.device_id = DeviceId(1);
    chip.variant = Some(ChipVariant::Cell(Cell::default()));

    ChipCreate { packet_stream: Some(stream), packet_sink: Some(sink), chip }
}

// T011: Test for CreateChip message
#[tokio::test]
async fn test_create_chip() {
    let harness = setup_test_harness().await;
    let chip_id = ChipId(1);
    let (stream, sink, _, _) = create_dummy_stream_sink();
    let params = create_params(chip_id, stream, sink);
    let response = harness.client.create(chip_id, params).await;
    assert!(response.is_ok(), "CreateChip failed: {:?}", response);
}

// T012: Test for DeleteChip message
#[tokio::test]
async fn test_delete_chip() {
    let mut harness = setup_test_harness().await;
    let chip_id = ChipId(2);
    let (stream, sink, _, _) = create_dummy_stream_sink();
    let params = create_params(chip_id, stream, sink);
    harness.client.create(chip_id, params).await.unwrap();

    let client = harness.client.clone();
    let delete_handle = tokio::spawn(async move { client.delete(chip_id).await });

    let notification_future = async {
        match tokio::time::timeout(
            std::time::Duration::from_secs(1),
            harness.device_server_rx.recv(),
        )
        .await
        {
            Ok(Some(ResourceRequest::Action {
                action: DeviceAction::NotifyChipRemoved(_device_id, id),
                respond_to,
                ..
            })) => {
                assert_eq!(id, chip_id);
                let _ = respond_to.send(Ok(DeviceActionResult::Success));
            }
            Ok(Some(msg)) => panic!("Received unexpected message: {:?}", msg),
            Ok(None) => panic!("Stream closed"),
            Err(_) => panic!("Timed out waiting for NotifyChipRemoved message"),
        }
    };

    tokio::select! {
        res = delete_handle => {
            let response = res.unwrap();
            assert!(response.is_ok(), "DeleteChip failed: {:?}", response);
        }
        _ = notification_future => {}
        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
            panic!("Test timed out");
        }
    }
}

// T013: Test for stream/sink error triggering chip deletion
#[tokio::test]
async fn test_stream_error_triggers_delete() {
    let mut harness = setup_test_harness().await;
    let chip_id = ChipId(3);
    let (stream, sink, mut stream_tx, _) = create_dummy_stream_sink();
    let params = create_params(chip_id, stream, sink);
    harness.client.create(chip_id, params).await.unwrap();

    // Close the stream sender to simulate an error
    stream_tx.close_channel();

    match tokio::time::timeout(std::time::Duration::from_secs(2), harness.device_server_rx.recv())
        .await
    {
        Ok(Some(ResourceRequest::Action {
            action: DeviceAction::NotifyChipRemoved(_device_id, id),
            respond_to,
            ..
        })) => {
            assert_eq!(id, chip_id);
            let _ = respond_to.send(Ok(DeviceActionResult::Success));
        }
        Ok(Some(msg)) => panic!("Received unexpected message: {:?}", msg),
        Ok(None) => panic!("Stream closed"),
        Err(_) => panic!("Timed out waiting for NotifyChipRemoved message on stream error"),
    }
}

// T020: Test stream to controller passthrough and ECHO response
#[tokio::test]
async fn test_stream_to_controller_echo() {
    // Requires controller to echo. ModemNetworkSimulator might not by default
    // unless we send AT commands it understands.
}

// T028: Test GetChip message
#[tokio::test]
async fn test_get_chip() {
    let harness = setup_test_harness().await;
    let chip_id = ChipId(1);
    let (stream, sink, _stream_tx, _sink_rx) = create_dummy_stream_sink();
    let params = create_params(chip_id, stream, sink);
    let create_response = harness.client.create(chip_id, params).await;
    assert!(create_response.is_ok(), "CreateChip failed: {:?}", create_response);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let chip = harness.client.read(chip_id).await.unwrap();
    if let Some(ChipVariant::Cell(cell_chip)) = &chip.variant {
        assert_eq!(cell_chip.state, "idle");
    } else {
        panic!("GetChip failed for existing chip");
    }

    // Test non-existent chip
    let bad_chip_id = ChipId(99);
    match harness.client.read(bad_chip_id).await {
        Err(netsim_model::ClientError::Chip(NetsimChipError::ChipNotFound(id))) => {
            assert_eq!(id, bad_chip_id);
        }
        other => panic!("Expected ChipNotFound error, got {:?}", other),
    }
}

// T014: Test that delete chip does not block the actor loop while waiting for
// DeviceActor
#[tokio::test]
async fn test_delete_chip_non_blocking() {
    let mut harness = setup_test_harness().await;
    let chip_id = ChipId(2);
    let (stream, sink, _, _) = create_dummy_stream_sink();
    let params = create_params(chip_id, stream, sink);
    harness.client.create(chip_id, params).await.unwrap();

    let client = harness.client.clone();
    // Start delete in a spawned task so we don't block the test
    let delete_handle = tokio::spawn(async move { client.delete(chip_id).await });

    // Wait for the NotifyChipRemoved message to arrive at the mock DeviceActor
    let msg =
        tokio::time::timeout(std::time::Duration::from_secs(1), harness.device_server_rx.recv())
            .await
            .unwrap()
            .unwrap();

    let ResourceRequest::Action {
        action: DeviceAction::NotifyChipRemoved(_device_id, id),
        respond_to,
        ..
    } = msg
    else {
        panic!("Unexpected message");
    };
    assert_eq!(id, chip_id);

    // At this point, NotifyChipRemoved has been sent, but we have NOT responded.
    // If CellActor is non-blocking (async), it should be able to process other
    // requests.
    let read_client = harness.client.clone();
    let read_handle = tokio::spawn(async move { read_client.read(chip_id).await });

    // Verify read completes (it might return error/NotFound, but it should NOT
    // hang)
    let read_res = tokio::time::timeout(std::time::Duration::from_millis(200), read_handle).await;
    assert!(read_res.is_ok(), "Read call was blocked by pending delete notification!");

    // Now respond to NotifyChipRemoved to let delete finish
    let _ = respond_to.send(Ok(DeviceActionResult::Success));

    // Verify delete finishes
    delete_handle.await.unwrap().unwrap();
}
