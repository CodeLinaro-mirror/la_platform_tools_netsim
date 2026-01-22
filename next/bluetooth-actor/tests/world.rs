// Copyright 2025 The Android Open Source Project

use actor_framework::ResourceClient;
use bluetooth_actor::{BluetoothActor, BluetoothClient};
use device_actor::client::DeviceClient;
use futures::sink::Sink;
use netsim_model::chip::{
    BeaconParams, BleBeacon, BluetoothCreate, BluetoothMode, ChipClient, ChipConfig, ChipCreate,
    ChipId, DeviceParams, NetworkParams, SnifferParams,
};
use netsim_model::device::DeviceId;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// The BDD World for Bluetooth Actor tests.
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
}

impl World {
    /// Creates a new World instance.
    pub fn new() -> Self {
        let (device_tx, _device_rx) = mpsc::channel(10);
        // We use a real ResourceClient for device_client since we don't need to mock it heavily yet,
        // but we could mock it if needed.
        let resource_client = DeviceClient::new(Box::new(ResourceClient::new(device_tx)));
        let (actor, client) = bluetooth_actor::new();
        let resource_client_clone = resource_client.clone();
        let client_clone = client.clone();
        let actor_task = tokio::spawn(async move {
            actor.run(BluetoothActor::new(resource_client_clone, client_clone)).await;
        });

        World {
            client,
            device_client: resource_client,
            _actor_task: actor_task,
            chip_id_counter: 0,
            device_id: DeviceId(1),
        }
    }

    /// Helper to get a fresh Chip ID.
    pub fn next_chip_id(&mut self) -> ChipId {
        self.chip_id_counter += 1;
        ChipId(self.chip_id_counter)
    }

    // --- Given Steps ---

    /// Creates a Bluetooth chip in Device mode.
    pub async fn given_bluetooth_device(&mut self) -> ChipId {
        let id = self.next_chip_id();
        let params = ChipCreate {
            id,
            packet_stream: None,
            packet_sink: None,
            config: ChipConfig::new(
                "device_chip",
                "netsim",
                "test_device",
                NetworkParams::Bluetooth(BluetoothCreate {
                    address: format!("00:00:00:00:00:{:02x}", id.0),
                    bt_properties: Default::default(),
                    mode: BluetoothMode::Device(DeviceParams {}),
                }),
            ),
            device_id: self.device_id,
        };
        self.client.0.create(params).await.expect("Failed to create device chip");
        id
    }

    /// Creates a Bluetooth chip in Beacon mode.
    pub async fn given_bluetooth_beacon(&mut self) -> ChipId {
        let id = self.next_chip_id();
        let params = ChipCreate {
            id,
            packet_stream: None,
            packet_sink: None,
            config: ChipConfig::new(
                "beacon_chip",
                "netsim",
                "test_beacon",
                NetworkParams::Bluetooth(BluetoothCreate {
                    address: format!("00:00:00:00:00:{:02x}", id.0),
                    bt_properties: Default::default(),
                    mode: BluetoothMode::Beacon(Box::new(BeaconParams {
                        ble_beacon: BleBeacon::default(),
                    })),
                }),
            ),
            device_id: self.device_id,
        };
        self.client.0.create(params).await.expect("Failed to create beacon chip");
        id
    }

    /// Creates a Bluetooth chip in Sniffer mode.
    pub async fn given_bluetooth_sniffer(
        &mut self,
        sink: Option<Pin<Box<dyn Sink<bytes::Bytes, Error = std::io::Error> + Send + Sync>>>,
    ) -> ChipId {
        let id = self.next_chip_id();
        let params = ChipCreate {
            id,
            packet_stream: None,
            packet_sink: sink,
            config: ChipConfig::new(
                "sniffer_chip",
                "netsim",
                "test_sniffer",
                NetworkParams::Bluetooth(BluetoothCreate {
                    address: format!("00:00:00:00:00:{:02x}", id.0),
                    bt_properties: Default::default(),
                    mode: BluetoothMode::Sniffer(SnifferParams {}),
                }),
            ),
            device_id: self.device_id,
        };
        self.client.0.create(params).await.expect("Failed to create sniffer chip");
        id
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
        id: ChipId,
    ) -> Result<(), actor_framework::FrameworkError> {
        self.client.0.delete(id).await
    }

    // --- Then Steps ---

    pub async fn then_chip_exists(&self, id: ChipId) {
        let chip = self.client.0.get(id).await.expect("Failed to get chip");
        assert!(chip.is_some(), "Chip {} should exist", id);
    }

    pub async fn then_chip_does_not_exist(&self, id: ChipId) {
        let chip = self.client.0.get(id).await.expect("Failed to get chip");
        assert!(chip.is_none(), "Chip {} should NOT exist", id);
    }

    pub async fn then_chip_count_is(&self, expected: usize) {
        let count = self.client.read_count_for_testing().await.expect("Failed to read chip count");
        assert_eq!(count, expected, "Chip count should be {}", expected);
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
