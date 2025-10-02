// Copyright 2023-2025 The Android Open Source Project

use crate::utils;
use ::bluetooth::manager::{BluetoothCommand, BluetoothManager};

use netsim_api::{
    BluetoothDeviceParams, ChipParams, CreateChipParams, DeleteChipParams, PacketStreamerApi,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{timeout, Duration};

/// A mock packet streamer that can be configured to return errors and has an
/// inbox for receiving packets.
#[derive(Clone)]
struct MockHciStreamer {
    should_error: Arc<AtomicBool>,
    packet_in_tx: mpsc::Sender<Vec<u8>>,
    packet_out_rx: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
    death_confirm_tx: Option<mpsc::Sender<()>>,
}

impl Drop for MockHciStreamer {
    fn drop(&mut self) {
        if let Some(tx) = self.death_confirm_tx.take() {
            let _ = tx.try_send(());
        }
    }
}

#[async_trait::async_trait]
impl PacketStreamerApi for MockHciStreamer {
    async fn read_packet(&mut self) -> Result<Option<Vec<u8>>, netsim_api::Error> {
        let packet = self
            .packet_out_rx
            .lock()
            .await
            .recv()
            .await
            .ok_or(netsim_api::Error::Packet("Channel closed".to_string()))?;

        if self.should_error.load(Ordering::SeqCst) {
            return Err(netsim_api::Error::Packet("Simulated packet stream error".to_string()));
        }

        Ok(Some(packet))
    }

    async fn write_packet(&mut self, packet: Vec<u8>) -> Result<(), netsim_api::Error> {
        self.packet_in_tx.send(packet).await.map_err(|e| netsim_api::Error::Packet(e.to_string()))
    }
}

#[tokio::test]
async fn test_hci_reset_command() {
    utils::setup_logging();

    let (bt_manager, command_tx) = BluetoothManager::new();
    tokio::spawn(async move {
        bt_manager.run().await;
    });

    let (packet_in_tx, mut packet_in_rx) = mpsc::channel(10);
    let (packet_out_tx, packet_out_rx) = mpsc::channel(10);

    let streamer = MockHciStreamer {
        should_error: Arc::new(AtomicBool::new(false)),
        packet_in_tx,
        packet_out_rx: Arc::new(Mutex::new(packet_out_rx)),
        death_confirm_tx: None,
    };

    // 1. Create a virtual device chip.
    let params = BluetoothDeviceParams { address: "AB:CD:EF:11:22:33".to_string() };
    let chip_params = ChipParams::BluetoothDevice(params);
    let (responder, rx) = oneshot::channel();
    let create_chip_params = CreateChipParams { chip_params, packet_streamer: Box::new(streamer) };
    command_tx
        .send(BluetoothCommand::CreateChip { params: create_chip_params, responder })
        .await
        .unwrap();
    let _chip_id = rx.await.unwrap().unwrap();

    // 2. Send an HCI Reset command.
    let hci_reset_cmd = vec![0x03, 0x0c, 0x00];
    packet_out_tx.send(hci_reset_cmd).await.unwrap();

    // 3. Wait for the HCI Command Complete event.
    let response = timeout(Duration::from_secs(1), packet_in_rx.recv()).await.unwrap().unwrap();

    // 4. Verify the response.
    // Expected: Command Complete for Reset, status OK.
    let expected_response = vec![0x0e, 0x04, 0x01, 0x03, 0x0c, 0x00];
    assert_eq!(response, expected_response);
}

#[tokio::test]
async fn test_chipper() {
    utils::setup_logging();
}

#[tokio::test]
async fn test_chip_dies_on_packet_stream_error() {
    utils::setup_logging();

    let (bt_manager, command_tx) = BluetoothManager::new();
    let bt_manager = Arc::new(bt_manager);
    let manager_clone = bt_manager.clone();
    tokio::spawn(async move {
        manager_clone.run().await;
    });

    let (death_confirm_tx, mut death_confirm_rx) = mpsc::channel(1);
    let (packet_in_tx, _packet_in_rx) = mpsc::channel(10);
    let (_packet_out_tx, packet_out_rx) = mpsc::channel(10);

    let streamer = MockHciStreamer {
        should_error: Arc::new(AtomicBool::new(false)),
        packet_in_tx,
        packet_out_rx: Arc::new(Mutex::new(packet_out_rx)),
        death_confirm_tx: Some(death_confirm_tx),
    };

    // 1. Create a virtual device chip.
    let params = BluetoothDeviceParams { address: "BE:EF:FA:CE:11:22".to_string() };
    let chip_params = ChipParams::BluetoothDevice(params);
    let (responder, rx) = oneshot::channel();
    let create_chip_params =
        CreateChipParams { chip_params, packet_streamer: Box::new(streamer.clone()) };
    command_tx
        .send(BluetoothCommand::CreateChip { params: create_chip_params, responder })
        .await
        .unwrap();
    let _chip_id = rx.await.unwrap().unwrap();

    // A small delay to ensure the chip is registered before we check the count.
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(bt_manager.get_chip_count_for_testing(), 1);

    // 2. Trigger a packet stream error.
    streamer.should_error.store(true, Ordering::SeqCst);
    // Unblock the packet streamer so it can encounter the error.
    let _ = _packet_out_tx.send(vec![]).await;

    // 3. The chip's background task should die and notify the manager.
    death_confirm_rx.recv().await;

    // 4. Verify the chip has been removed.
    // A small delay is needed to ensure the manager has time to process the death notice.
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(bt_manager.get_chip_count_for_testing(), 0);
}

#[tokio::test]
async fn test_delete_chip_shuts_down_task() {
    utils::setup_logging();

    let (bt_manager, command_tx) = BluetoothManager::new();
    let bt_manager = Arc::new(bt_manager);
    let manager_clone = bt_manager.clone();
    tokio::spawn(async move {
        manager_clone.run().await;
    });

    let (death_confirm_tx, mut death_confirm_rx) = mpsc::channel(1);
    let (packet_in_tx, _packet_in_rx) = mpsc::channel(10);
    let (packet_out_tx, packet_out_rx) = mpsc::channel(10);

    let streamer = MockHciStreamer {
        should_error: Arc::new(AtomicBool::new(false)),
        packet_in_tx,
        packet_out_rx: Arc::new(Mutex::new(packet_out_rx)),
        death_confirm_tx: Some(death_confirm_tx),
    };

    // 1. Create a virtual device chip.
    let params = BluetoothDeviceParams { address: "DE:AD:BE:EF:33:44".to_string() };
    let chip_params = ChipParams::BluetoothDevice(params);
    let (responder, rx) = oneshot::channel();
    let create_chip_params = CreateChipParams { chip_params, packet_streamer: Box::new(streamer) };
    command_tx
        .send(BluetoothCommand::CreateChip { params: create_chip_params, responder })
        .await
        .unwrap();
    let chip_id = rx.await.unwrap().unwrap();

    assert_eq!(bt_manager.get_chip_count_for_testing(), 1);

    // 2. Send a DeleteChip command.
    let (responder, rx) = oneshot::channel();
    let delete_params = DeleteChipParams { chip_id };
    command_tx
        .send(BluetoothCommand::DeleteChip { params: delete_params, responder })
        .await
        .unwrap();
    rx.await.unwrap().unwrap();

    // 3. The chip's background task should receive the shutdown signal and exit.
    death_confirm_rx.recv().await;

    // 4. Verify the chip has been removed.
    assert_eq!(bt_manager.get_chip_count_for_testing(), 0);
}
