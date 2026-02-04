// Copyright 2025 The Android Open Source Project

use actor_framework::ResourceClient;
use bluetooth_actor::{BluetoothActor, BluetoothClient};
use device_actor::client::DeviceClient;
use netsim_model::chip::{
    BeaconParams, BleBeacon, BluetoothCreate, BluetoothMode, ChipConfig, ChipCreate, ChipId,
    DeviceParams, NetworkParams, SnifferParams,
};
use netsim_model::device::DeviceId;
use netsim_testing::logger;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// The BDD World for Bluetooth Actor tests.
#[allow(dead_code)]
pub struct World {
    /// The Bluetooth Client under test.
    pub client: BluetoothClient,
    /// The mocked or real DeviceClient for interactions.
    pub device_client: DeviceClient,
    /// The background task running the actor (dropped on World drop).
    pub _actor_task: JoinHandle<()>,
    /// Counter for generating unique ChipIds in tests.
    pub chip_id_counter: u32,
    /// The ID of the simulated device.
    pub device_id: DeviceId,
    /// Map of chip names to their IDs.
    pub chips: std::collections::HashMap<String, ChipId>,
    /// Map of chip names to their stream senders (to inject packets).
    pub streams: std::collections::HashMap<String, mpsc::Sender<bytes::Bytes>>,
    /// Map of chip names to their sink receivers (to capture packets).
    pub sinks: std::collections::HashMap<String, mpsc::Receiver<Vec<u8>>>,
}

#[allow(dead_code)]
impl World {
    /// Creates a new World instance.
    pub fn new() -> Self {
        logger::setup(None);
        let (device_tx, _device_rx) = mpsc::channel(10);
        // Use real DeviceClient for integration testing.
        let resource_client = DeviceClient::new(Box::new(ResourceClient::new(device_tx)));
        let (actor, client) = bluetooth_actor::new();
        let resource_client_clone = resource_client.clone();
        let _client_clone = client.clone();
        let actor_task = tokio::spawn(async move {
            actor.run(BluetoothActor::new(resource_client_clone)).await;
        });

        World {
            client,
            device_client: resource_client,
            _actor_task: actor_task,
            chip_id_counter: 0,
            device_id: DeviceId(1),
            chips: std::collections::HashMap::new(),
            streams: std::collections::HashMap::new(),
            sinks: std::collections::HashMap::new(),
        }
    }

    /// Helper to get a fresh Chip ID.
    pub fn next_chip_id(&mut self) -> ChipId {
        self.chip_id_counter += 1;
        ChipId(self.chip_id_counter)
    }

    // --- Given Steps ---

    /// Creates a Bluetooth chip in Device mode with a given name.
    /// The stream and sink are automatically created and managed by the World.
    pub async fn given_bluetooth_device(&mut self, name: &str) {
        if self.chips.contains_key(name) {
            panic!("Chip with name '{}' already exists", name);
        }

        let (stream, stream_tx) = crate::test_utils::mock_stream();
        let (sink, sink_rx) = crate::test_utils::mock_sink();

        let id = self.next_chip_id();
        let params = ChipCreate {
            id,
            packet_stream: Some(stream),
            packet_sink: Some(sink),
            config: ChipConfig::new(
                "device_chip",
                "netsim",
                name,
                NetworkParams::Bluetooth(BluetoothCreate {
                    address: format!("00:00:00:00:00:{:02x}", id.0),
                    bt_properties: Default::default(),
                    mode: BluetoothMode::Device(DeviceParams {}),
                }),
            ),
            device_id: self.device_id,
        };
        self.client.0.create(params).await.expect("Failed to create device chip");

        self.chips.insert(name.to_string(), id);
        self.streams.insert(name.to_string(), stream_tx);
        self.sinks.insert(name.to_string(), sink_rx);
    }

    /// Creates a Bluetooth chip in Beacon mode.
    pub async fn given_bluetooth_beacon(&mut self, name: &str) {
        let chip_id = self.next_chip_id().0;
        self.given_bluetooth_beacon_with_address(name, &format!("00:00:00:00:00:{:02x}", chip_id))
            .await
    }

    /// Creates a Bluetooth chip in Beacon mode with a specific address.
    pub async fn given_bluetooth_beacon_with_address(&mut self, name: &str, address: &str) {
        let id = self.next_chip_id();
        let params = ChipCreate {
            id,
            packet_stream: None,
            packet_sink: None,
            config: ChipConfig::new(
                "beacon_chip",
                "netsim",
                name,
                NetworkParams::Bluetooth(BluetoothCreate {
                    address: address.to_string(),
                    bt_properties: Default::default(),
                    mode: BluetoothMode::Beacon(Box::new(BeaconParams {
                        ble_beacon: BleBeacon {
                            address: address.to_string(),
                            ..Default::default()
                        },
                    })),
                }),
            ),
            device_id: self.device_id,
        };
        self.client.0.create(params).await.expect("Failed to create beacon chip");
        self.chips.insert(name.to_string(), id);
    }

    /// Creates a Bluetooth chip in Sniffer mode.
    pub async fn given_bluetooth_sniffer(&mut self, name: &str) {
        if self.chips.contains_key(name) {
            panic!("Chip with name '{}' already exists", name);
        }

        let (sink, sink_rx) = crate::test_utils::mock_sink();

        let id = self.next_chip_id();
        let params = ChipCreate {
            id,
            packet_stream: None,
            packet_sink: Some(sink),
            config: ChipConfig::new(
                "sniffer_chip",
                "netsim",
                name,
                NetworkParams::Bluetooth(BluetoothCreate {
                    address: format!("00:00:00:00:00:{:02x}", id.0),
                    bt_properties: Default::default(),
                    mode: BluetoothMode::Sniffer(SnifferParams {}),
                }),
            ),
            device_id: self.device_id,
        };
        self.client.0.create(params).await.expect("Failed to create sniffer chip");
        self.chips.insert(name.to_string(), id);
        self.sinks.insert(name.to_string(), sink_rx);
    }

    // --- When Steps ---

    pub async fn when_create_chip(
        &self,
        params: ChipCreate,
    ) -> Result<(), actor_framework::FrameworkError> {
        self.client.0.create(params).await.map(|_| ())
    }

    pub async fn when_delete_chip(
        &self,
        name: &str,
    ) -> Result<(), actor_framework::FrameworkError> {
        let id = *self.chips.get(name).expect("Chip not found");
        self.client.0.delete(id).await
    }

    /// Drops the stream sender for the given chip, simulating a stream closure.
    pub fn when_stream_dropped(&mut self, name: &str) {
        self.streams.remove(name).expect("Stream not found for chip");
    }

    /// Drops the sink receiver for the given chip, simulating a sink closure.
    pub fn when_sink_dropped(&mut self, name: &str) {
        self.sinks.remove(name).expect("Sink not found for chip");
    }

    pub async fn when_update_chip_position(
        &self,
        name: &str,
        position: netsim_model::device::Position,
    ) {
        let id = *self.chips.get(name).expect("Chip not found");
        let update =
            netsim_model::chip::ChipUpdate { position: Some(position), ..Default::default() };
        self.client.0.update(id, update).await.expect("Failed to update chip");
    }

    // --- Then Steps ---

    pub async fn then_chip_position_is(
        &self,
        name: &str,
        expected: netsim_model::device::Position,
    ) {
        let id = *self.chips.get(name).expect("Chip not found");
        let chip =
            self.client.0.get(id).await.expect("Failed to get chip").expect("Chip should exist");
        assert_eq!(chip.position, expected, "Chip position matches");
    }

    pub async fn then_chip_exists(&self, name: &str) {
        let id = *self.chips.get(name).expect("Chip name tracked in World");
        let chip = self.client.0.get(id).await.expect("Failed to get chip");
        assert!(chip.is_some(), "Chip {} ({}) should exist", name, id);
    }

    pub async fn then_chip_does_not_exist(&self, name: &str) {
        if let Some(id) = self.chips.get(name) {
            let chip = self.client.0.get(*id).await.expect("Failed to get chip");
            assert!(chip.is_none(), "Chip {} ({}) should NOT exist", name, id);
        }
    }

    pub async fn then_chip_count_is(&self, expected: usize) {
        let count = self.client.0.list().await.expect("Failed to list chips").len();
        assert_eq!(count, expected, "Chip count should be {}", expected);
    }

    pub async fn when_packet_sent(&mut self, name: &str, packet: bytes::Bytes) {
        let tx = self.streams.get_mut(name).expect("Stream not found for chip");
        tx.send(packet).await.expect("Failed to send packet");
    }

    pub async fn then_packet_received(&mut self, name: &str, expected: &[u8]) {
        let actual = self.receive_packet(name).await;
        assert_eq!(actual, expected, "Packet received by {} does not match expected", name);
    }

    pub async fn receive_packet(&mut self, name: &str) -> Vec<u8> {
        let rx = self.sinks.get_mut(name).expect("Sink not found for chip");
        tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("Timed out waiting for packet")
            .expect("Packet stream closed unexpectedly")
    }

    /// Helper to create a ChipConfig.
    pub fn create_chip_config(id: ChipId, mode: BluetoothMode) -> ChipConfig {
        ChipConfig::new(
            "test_chip",
            "test_manufacturer",
            "test_product",
            NetworkParams::Bluetooth(BluetoothCreate {
                address: format!("00:00:00:00:00:{:02x}", id.0),
                bt_properties: Default::default(),
                mode,
            }),
        )
    }
}
