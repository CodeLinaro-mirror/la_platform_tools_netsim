// Copyright 2023 The Android Open Source Project

//! Test for verifying the exchange of HCI packets with the rootcanal controller.

use env_logger;
use log::error;
use rootcanal::{
    bluetooth::{Bluetooth, Callbacks as RootcanalCallbacks},
    controller::{Callbacks as ControllerCallbacks, Id, Idc},
    types::{Address, Phy},
};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Once;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout, Duration};

static INIT: Once = Once::new();

/// Set up the logger for the test.
fn setup() {
    INIT.call_once(env_logger::init);
}

/// Main callbacks for the rootcanal Bluetooth instance.
struct TestCallbacks;
impl RootcanalCallbacks for TestCallbacks {
    fn on_send_ll(
        &self,
        _source_id: u32,
        _destination_id: u32,
        _packet: &[u8],
        _phy: Phy,
        tx_power: i32,
    ) -> Option<i32> {
        Some(tx_power)
    }
}

/// Callbacks for the sniffer controller, which forwards received LL packets.
struct SnifferCallbacks {
    sender: mpsc::Sender<Vec<u8>>,
}

impl ControllerCallbacks for SnifferCallbacks {
    fn send_hci(&self, _source_id: Id, _idc: Idc, _data: &[u8]) {}
    fn on_receive_ll(&self, _source_id: Id, packet: &[u8], _phy: Phy, _tx_power: i32) {
        let _ = self.sender.try_send(packet.to_vec());
    }
    fn invalid_packet_received(&self, source_id: Id, reason: i32, message: &str, data: &[u8]) {
        error!(
            "sniffer invalid packet from {}: reason={}, message='{}', data={:02x?}",
            source_id, reason, message, data
        );
    }
}

/// Callbacks for the dummy controller, which forwards received HCI events.
struct DummyCallbacks {
    sender: mpsc::Sender<Vec<u8>>,
}

impl ControllerCallbacks for DummyCallbacks {
    fn send_hci(&self, _source_id: Id, idc: Idc, data: &[u8]) {
        if idc == Idc::Evt {
            let _ = self.sender.try_send(data.to_vec());
        }
    }
    fn on_receive_ll(&self, _source_id: Id, _packet: &[u8], _phy: Phy, _tx_power: i32) {}
    fn invalid_packet_received(&self, source_id: Id, reason: i32, message: &str, data: &[u8]) {
        error!(
            "dummy invalid packet from {}: reason={}, message='{}', data={:02x?}",
            source_id, reason, message, data
        );
    }
}

/// Asynchronously receive an HCI event from the controller.
async fn get_hci_event(receiver: &mut mpsc::Receiver<Vec<u8>>) -> Vec<u8> {
    timeout(Duration::from_secs(1), receiver.recv()).await.unwrap().unwrap()
}

/// Assert that a Command Complete event is received for the given opcode.
async fn assert_command_complete(receiver: &mut mpsc::Receiver<Vec<u8>>, lsb: u8, msb: u8) {
    let event = get_hci_event(receiver).await;
    assert_eq!(event[0], 0x0e); // Command Complete
    assert_eq!(event[3], lsb); // OpCode LSB
    assert_eq!(event[4], msb); // OpCode MSB
    assert_eq!(event[5], 0x00); // Status OK
}

/// Test case for sending a sequence of HCI commands to the rootcanal controller
/// and verifying that the correct advertising packet is sent over the air.
// Verifies the end-to-end flow of sending HCI commands to a controller and
// ensuring the correct link-layer advertising packet is generated and transmitted.
// This test simulates a Bluetooth host by:
// 1. Creating two controllers: one to act as the device under test (DUT) and
//    another to act as a sniffer.
// 2. Sending a sequence of HCI commands to the DUT to configure and enable
//    legacy advertising (Reset, Set Adv Params, Set Adv Data, Set Adv Enable).
// 3. Verifying that the DUT responds with a Command Complete event for each command.
// 4. Capturing the resulting link-layer packet from the sniffer.
// 5. Parsing the captured packet as a LeLegacyAdvertisingPdu and asserting that
//    its contents (source address, advertising data) match the parameters
//    configured via HCI.
#[tokio::test]
async fn test_hci_exchange() {
    setup();
    let rootcanal = Arc::new(Bluetooth::new(Box::new(TestCallbacks)));
    let (ll_sender, mut ll_receiver) = mpsc::channel(10);
    let (hci_sender, mut hci_receiver) = mpsc::channel(10);

    let rootcanal_clone = rootcanal.clone();
    tokio::spawn(async move {
        loop {
            rootcanal_clone.tick();
            sleep(Duration::from_millis(20)).await;
        }
    });

    // Create a sniffer to listen for packets.
    let sniffer_address = Address::from_str("00:00:00:00:00:01").unwrap();
    let sniffer_id = 1;
    rootcanal
        .new_controller(
            sniffer_id,
            sniffer_address,
            Box::new(SnifferCallbacks { sender: ll_sender }),
        )
        .expect("new controller failed");

    // Create a controller to send the HCI commands.
    let sender_address = Address::from_str("00:00:00:00:00:02").unwrap();
    let sender_id = 2;
    rootcanal
        .new_controller(sender_id, sender_address, Box::new(DummyCallbacks { sender: hci_sender }))
        .expect("new controller failed");

    // Reset the controller first.
    let reset_cmd = vec![0x03, 0x0c, 0x00];
    rootcanal.receive_hci(sender_id, Idc::Cmd, &reset_cmd).unwrap();
    assert_command_complete(&mut hci_receiver, 0x03, 0x0c).await;

    // LE Set Advertising Parameters
    let adv_params = vec![
        0x06, 0x20, 15, 0xA0, 0x00, 0xA0, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x07, 0x00,
    ];
    rootcanal.receive_hci(sender_id, Idc::Cmd, &adv_params).unwrap();
    assert_command_complete(&mut hci_receiver, 0x06, 0x20).await;

    // LE Set Advertising Data
    let mut adv_data = vec![0x08, 0x20, 32];
    adv_data.push(3); // Advertising_Data_Length
    adv_data.extend_from_slice(&[0x02, 0x01, 0x06]); // Advertising_Data
    adv_data.resize(3 + 32, 0); // Pad to 32 bytes of parameters
    rootcanal.receive_hci(sender_id, Idc::Cmd, &adv_data).unwrap();
    assert_command_complete(&mut hci_receiver, 0x08, 0x20).await;

    // LE Set Advertising Enable
    let adv_enable = vec![0x0A, 0x20, 0x01, 0x01];
    rootcanal.receive_hci(sender_id, Idc::Cmd, &adv_enable).unwrap();
    assert_command_complete(&mut hci_receiver, 0x0A, 0x20).await;

    // Check for advertising packet
    let packet = timeout(Duration::from_secs(5), ll_receiver.recv()).await.unwrap().unwrap();

    // Parse the packet using netsim_packets
    use netsim_packets::link_layer::{Address as LlAddress, LeLegacyAdvertisingPdu};
    use pdl_runtime::Packet;
    let parsed = LeLegacyAdvertisingPdu::decode(&packet).unwrap().0;

    // Verify the advertising PDU
    let mut expected_addr = [0; 8];
    expected_addr[..6].copy_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x02]);
    assert_eq!(
        parsed.source_address(),
        LlAddress::try_from(u64::from_le_bytes(expected_addr)).unwrap()
    );
    assert_eq!(parsed.advertising_data(), &[0x02, 0x01, 0x06]);
}
