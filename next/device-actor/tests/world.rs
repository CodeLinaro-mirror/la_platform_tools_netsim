use std::{
    collections::HashMap,
    sync::{atomic::AtomicU32, Arc},
};

use bytes::Bytes;
use device_actor::{DeviceActor, DeviceClient};
use device_api::{
    api::{DeviceChipCreate, DeviceCreate},
    DeviceConfig, DeviceId,
};
use futures::{SinkExt, StreamExt};
use link_api::MockLinkClient;
use netsim_model::chip::{
    BluetoothUpdate, ChipClient, ChipKind, ChipUpdate, ChipVariantUpdate, MockChipClient,
    RadioUpdate,
};

/// The BDD World for Device Actor tests.
pub struct World {
    pub client: DeviceClient,
    _actor_task: tokio::task::JoinHandle<()>,
    // Shared state for radio stats, injected into mock chips
    pub radio_stats: Arc<std::sync::Mutex<Vec<netsim_model::stats::NetsimRadioStats>>>,
    // Last fetched radio stats
    pub last_radio_stats: Option<Vec<netsim_model::stats::NetsimRadioStats>>,
    // Path to clean up on drop
    stats_file_to_cleanup: Option<std::path::PathBuf>,
    // BDD State
    pub current_device_id: Option<DeviceId>,
    pub current_chip_id: Option<netsim_model::chip::ChipId>,
    pub transport_tx: Option<tokio::sync::mpsc::UnboundedSender<Bytes>>,
}

impl Drop for World {
    fn drop(&mut self) {
        self._actor_task.abort();
        if let Some(path) = &self.stats_file_to_cleanup {
            if path.exists() {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

impl World {
    /// Creates a new World with default mock clients.
    pub async fn new() -> Self {
        let radio_stats = Arc::new(std::sync::Mutex::new(Vec::new()));
        let chip_clients = Self::create_default_chip_clients_with_stats(radio_stats.clone());
        let link_client = Self::create_default_link_client();
        let (stats_path, _) = Self::temp_stats_path();
        Self::with_clients_internal(
            chip_clients,
            link_client,
            None,
            None,
            Some(stats_path.clone()),
            None,
            radio_stats,
            Some(stats_path),
        )
        .await
    }

    pub async fn new_with_stats(
        path: std::path::PathBuf,
        interval: Option<std::time::Duration>,
    ) -> Self {
        let radio_stats = Arc::new(std::sync::Mutex::new(Vec::new()));
        let chip_clients = Self::create_default_chip_clients_with_stats(radio_stats.clone());
        let link_client = Self::create_default_link_client();
        Self::with_clients_internal(
            chip_clients,
            link_client,
            None,
            None,
            Some(path),
            interval,
            radio_stats,
            None,
        )
        .await
    }

    /// Creates a new World with injected custom mock clients.
    pub async fn with_clients(
        chip_clients: HashMap<ChipKind, Box<dyn ChipClient>>,
        link_client: MockLinkClient,
    ) -> Self {
        let radio_stats = Arc::new(std::sync::Mutex::new(Vec::new()));
        let (stats_path, _) = Self::temp_stats_path();
        Self::with_clients_internal(
            chip_clients,
            link_client,
            None,
            None,
            Some(stats_path.clone()),
            None,
            radio_stats,
            Some(stats_path),
        )
        .await
    }

    /// Creates a new World with injected custom mock clients and idle timeout.
    pub async fn with_clients_and_timeout(
        chip_clients: HashMap<ChipKind, Box<dyn ChipClient>>,
        link_client: MockLinkClient,
        startup_timeout: Option<std::time::Duration>,
        idle_timeout: Option<std::time::Duration>,
        stats_path: Option<std::path::PathBuf>,
    ) -> Self {
        let radio_stats = Arc::new(std::sync::Mutex::new(Vec::new()));

        // If stats_path is None, create a temp one to avoid pollution
        let (final_path, cleanup_path) = if let Some(p) = stats_path {
            (Some(p), None)
        } else {
            let (p, _) = Self::temp_stats_path();
            (Some(p.clone()), Some(p))
        };

        Self::with_clients_internal(
            chip_clients,
            link_client,
            startup_timeout,
            idle_timeout,
            final_path,
            None,
            radio_stats,
            cleanup_path,
        )
        .await
    }

    /// Internal helper to create World
    async fn with_clients_internal(
        chip_clients: HashMap<ChipKind, Box<dyn ChipClient>>,
        link_client: MockLinkClient,
        startup_timeout: Option<std::time::Duration>,
        idle_timeout: Option<std::time::Duration>,
        stats_path: Option<std::path::PathBuf>,
        stats_interval: Option<std::time::Duration>,
        radio_stats: Arc<std::sync::Mutex<Vec<netsim_model::stats::NetsimRadioStats>>>,
        stats_file_to_cleanup: Option<std::path::PathBuf>,
    ) -> Self {
        let (runner, client) = device_actor::new();
        let actor = DeviceActor::new(
            chip_clients,
            Arc::new(AtomicU32::new(0)),
            None,
            Box::new(link_client),
            startup_timeout,
            idle_timeout,
            "0.0.0-test".to_string(),
            stats_path,
            stats_interval,
        );
        let actor_task = tokio::spawn(runner.run(actor));
        World {
            client,
            _actor_task: actor_task,
            radio_stats,
            last_radio_stats: None,
            stats_file_to_cleanup,
            current_device_id: None,
            current_chip_id: None,
            transport_tx: None,
        }
    }

    pub fn create_default_chip_clients() -> HashMap<ChipKind, Box<dyn ChipClient>> {
        let radio_stats = Arc::new(std::sync::Mutex::new(Vec::new()));
        Self::create_default_chip_clients_with_stats(radio_stats)
    }

    pub fn create_default_chip_clients_with_stats(
        radio_stats: Arc<std::sync::Mutex<Vec<netsim_model::stats::NetsimRadioStats>>>,
    ) -> HashMap<ChipKind, Box<dyn ChipClient>> {
        let mut clients: HashMap<ChipKind, Box<dyn ChipClient>> = HashMap::new();
        // Add default mocks for common chip kinds
        for kind in [ChipKind::BLUETOOTH, ChipKind::WIFI, ChipKind::UWB] {
            clients.insert(kind, Box::new(Self::create_default_mock_chip(radio_stats.clone())));
        }
        clients
    }

    pub(crate) fn create_default_mock_chip(
        radio_stats: Arc<std::sync::Mutex<Vec<netsim_model::stats::NetsimRadioStats>>>,
    ) -> MockChipClient {
        let mut mock = MockChipClient::new();
        mock.expect_read().returning(|_| Ok(netsim_model::chip::Chip::default()));
        mock.expect_update().returning(|_, _| Ok(netsim_model::chip::Chip::default()));
        mock.expect_update().returning(|_, _| Ok(netsim_model::chip::Chip::default()));
        mock.expect_create().returning(|params| {
            if let Some(mut stream) = params.packet_stream {
                tokio::spawn(async move { while stream.next().await.is_some() {} });
            }
            Ok(())
        });
        mock.expect_delete().returning(|_| Ok(()));

        // Return stats from shared state
        let rs_for_read = radio_stats.clone();
        mock.expect_read_statistics().returning(move || {
            let stats = rs_for_read.lock().unwrap();
            Ok(Box::from(stats.clone()))
        });

        mock.expect_reset().returning(|_| Ok(()));
        // Clone returns a fresh mock handling stats from the shared state
        let radio_stats_clone = radio_stats.clone();
        mock.expect_clone_box().returning(move || {
            let mut clone_mock = MockChipClient::new();
            let rs_clone = radio_stats_clone.clone();

            clone_mock.expect_read_statistics().returning(move || {
                let stats = rs_clone.lock().unwrap();
                Ok(Box::from(stats.clone()))
            });

            Box::new(clone_mock)
        });
        mock
    }

    pub fn create_default_link_client() -> MockLinkClient {
        let mut mock = MockLinkClient::new();
        mock.expect_action().returning(|_, _| Ok(()));
        mock.expect_create().returning(|_| Ok(link_api::LinkId(0)));
        mock.expect_update().returning(|_, _| Ok(())); // update returns Result<(), String>
        mock.expect_delete().returning(|_| Ok(())); // delete returns Result<(), String>
        mock.expect_list().returning(|| Ok(vec![])); // list returns Result<Vec<Link>, String>
        mock.expect_notify_chip_added().returning(|_, _| Ok(()));
        mock.expect_notify_chip_removed().returning(|_| Ok(()));
        mock
    }

    /// BDD Step: When I create a new device.
    pub async fn when_create_device(&self, name: &str) -> DeviceId {
        let params = DeviceCreate {
            device_config: DeviceConfig::new(
                name.to_string(),
                true,
                Default::default(),
                Default::default(),
                false,
            ),
            chip: DeviceChipCreate {
                name: "beacon".to_string(),
                manufacturer: "Netsim".to_string(),
                product_name: "NetsimBeacon".to_string(),
                chip: device_api::api::Chip::Beacon(Default::default()),
            },
        };
        self.client.create_device(params).await.unwrap()
    }

    /// BDD Step: When I add a chip (which creates a device).
    pub async fn when_add_chip(&self, device_guid: &str, chip_name: &str) -> DeviceId {
        let params = Self::create_device_add_chip_params(
            device_guid.to_string(),
            chip_name.to_string(),
            "00:00:00:00:00:00".to_string(),
        );
        self.client.add_chip(params).await.unwrap()
    }

    /// BDD Step: When I update the device.
    pub async fn when_update_device(
        &self,
        device_id: DeviceId,
        update: device_api::api::DeviceUpdate,
    ) {
        self.client.update(device_id, update).await.unwrap();
    }

    /// BDD Step: When I update the device with a specific chip update.
    pub async fn when_update_device_chip(&self, device_id: DeviceId, chip_update: ChipUpdate) {
        let mut update = device_api::api::DeviceUpdate::default();
        update.id = device_id.0;
        update.chips = Some(vec![chip_update]);
        self.when_update_device(device_id, update).await;
    }

    /// BDD Step: When I notify that a chip was removed.
    pub async fn when_notify_chip_removed(
        &self,
        device_id: DeviceId,
        chip_id: netsim_model::chip::ChipId,
    ) {
        self.client.notify_chip_removed(device_id, chip_id).await.unwrap();
    }

    /// BDD Step: When I shut down the actor politely to trigger shutdown hooks
    pub async fn when_shutdown_actor(&self) {
        let _ = self.client.shutdown().await;
        // Wait for Actor loop to flush and execute on_shutdown
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    /// BDD Step: When I delete the device.
    pub async fn when_delete_device(&self, id: DeviceId) {
        self.client.delete(id).await.unwrap();
    }

    /// BDD Step: When I add two chips with the same device GUID concurrently
    pub async fn when_concurrently_add_chips(
        &self,
        device_guid: &str,
        chip_name_1: &str,
        chip_name_2: &str,
    ) -> (DeviceId, DeviceId) {
        let client1 = self.client.clone();
        let client2 = self.client.clone();
        let guid1 = device_guid.to_string();
        let guid2 = device_guid.to_string();
        let name1 = chip_name_1.to_string();
        let name2 = chip_name_2.to_string();

        let t1 = tokio::spawn(async move {
            let params =
                Self::create_device_add_chip_params(guid1, name1, "00:00:00:00:00:01".to_string());
            client1.add_chip(params).await.unwrap()
        });

        let t2 = tokio::spawn(async move {
            let params =
                Self::create_device_add_chip_params(guid2, name2, "00:00:00:00:00:02".to_string());
            client2.add_chip(params).await.unwrap()
        });

        let (res1, res2) = tokio::join!(t1, t2);
        (res1.unwrap(), res2.unwrap())
    }

    /// BDD Step: When I add a chip with an injected packet stream.
    /// Returns the sending end of the channel to simulate transport traffic.
    pub async fn when_add_chip_with_stream(
        &self,
        device_guid: &str,
        chip_name: &str,
    ) -> (DeviceId, tokio::sync::mpsc::UnboundedSender<Bytes>) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
        let boxed_stream: netsim_model::chip::PacketStream = Box::new(stream);
        let sink = futures::sink::drain()
            .sink_map_err(|_| std::io::Error::from(std::io::ErrorKind::Other));
        let boxed_sink: netsim_model::chip::PacketSink = Box::pin(sink);

        let mut params = Self::create_device_add_chip_params(
            device_guid.to_string(),
            chip_name.to_string(),
            "00:00:00:00:00:00".to_string(),
        );
        params.packet_stream = Some(boxed_stream);
        params.packet_sink = Some(boxed_sink);

        let device_id = self.client.add_chip(params).await.unwrap();
        (device_id, tx)
    }

    /// Checks if the actor task has finished (e.g. due to shutdown).
    pub fn is_actor_finished(&self) -> bool {
        self._actor_task.is_finished()
    }

    /// Helper to create DeviceAddChip params with defaults.
    pub fn create_device_add_chip_params(
        device_guid: String,
        chip_name: String,
        chip_address: String,
    ) -> device_api::DeviceAddChip {
        device_api::DeviceAddChip {
            device_guid,
            packet_stream: None,
            packet_sink: None,
            device_config: DeviceConfig::new(
                "test-dev".to_string(),
                true,
                Default::default(),
                Default::default(),
                false,
            ),
            chip_config: netsim_model::chip::ChipConfig {
                name: chip_name,
                manufacturer: "Netsim".to_string(),
                product_name: "NetsimBeacon".to_string(),
                chip_kind_params: netsim_model::chip::ChipKindParams::Bluetooth(
                    netsim_model::chip::BluetoothCreate {
                        address: chip_address,
                        bt_properties: Default::default(),
                        mode: netsim_model::chip::BluetoothMode::Device(Default::default()),
                    },
                ),
            },
        }
    }
    /// Creates a Bluetooth ChipUpdate with the specified Low Energy and Classic
    /// radio states.
    pub fn create_bluetooth_chip_update(
        le_state: Option<bool>,
        classic_state: Option<bool>,
    ) -> ChipUpdate {
        ChipUpdate {
            variant: Some(ChipVariantUpdate::Bluetooth(BluetoothUpdate {
                low_energy: RadioUpdate { state: le_state },
                classic: RadioUpdate { state: classic_state },
            })),
            ..Default::default()
        }
    }

    /// Helper to get a unique temporary path for stats.
    pub fn temp_stats_path() -> (std::path::PathBuf, String) {
        let mut path = std::env::temp_dir();
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let filename = format!("netsim_session_stats_{}.json", unique_id);
        path.push(&filename);
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        (path, filename)
    }

    /// Verifies the content of the stats file.
    pub async fn verify_stats_file_content(
        path: &std::path::PathBuf,
        expected_version: &str,
        expected_device_count: u32,
        expected_peak_devices: u32,
    ) {
        let mut last_content = String::new();
        let mut found = false;
        // Wait up to 5 seconds
        for _ in 0..50 {
            if path.exists() {
                if let Ok(c) = std::fs::read_to_string(path) {
                    if !c.is_empty() {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&c) {
                            // Check device_count if it matches.
                            // Note: device_count in proto is cumulative.
                            // Protobuf JSON mapping uses camelCase.
                            let count = json["deviceCount"].as_u64().unwrap_or(0);
                            if count == expected_device_count as u64 {
                                found = true;
                                last_content = c;
                                break;
                            }
                        }
                        last_content = c;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        assert!(found, "Stats file content mismatch or timeout. Last content: {}", last_content);

        let json: serde_json::Value =
            serde_json::from_str(&last_content).expect("Failed to parse stats JSON");
        assert_eq!(json["version"], expected_version, "version mismatch");

        let val = json["deviceCount"].as_u64().unwrap_or(0);
        assert_eq!(val, expected_device_count as u64, "device_count mismatch");

        let val = json["peakConcurrentDevices"].as_u64().unwrap_or(0);
        assert_eq!(val, expected_peak_devices as u64, "peak_concurrent_devices mismatch");

        // precise mapping depends on proto compiler options, check both snake and camel
        let duration = json["duration_secs"]
            .as_u64()
            .or_else(|| json["durationSecs"].as_u64())
            .or_else(|| json["durationSecs"].as_str().map(|s| s.parse::<u64>().unwrap_or(0)));

        assert!(duration.is_some(), "duration_secs should be present");
    }

    /// BDD Step: Given a device with a transport stream
    pub async fn given_device_with_transport_stream(&mut self, device_guid: &str, chip_name: &str) {
        let (device_id, tx) = self.when_add_chip_with_stream(device_guid, chip_name).await;
        self.current_device_id = Some(device_id);
        self.transport_tx = Some(tx);

        // Fetch Chip ID
        let device =
            self.client.get(device_id).await.expect("RPC failed").expect("Device not found");
        let chip_id =
            device.chips.first().expect("Device created with transport stream has no chips").id;
        self.current_chip_id = Some(netsim_model::chip::ChipId(chip_id));
    }

    /// BDD Step: Given mock radio stats are primed
    pub fn given_mock_radio_stats(&self, tx: u64, rx: u64) {
        let chip_id = self.current_chip_id.expect("No current chip set in World").0;
        self.given_radio_stats(chip_id, tx, rx);
    }

    /// BDD Step: When I send packets to the transport
    pub async fn when_send_packets_to_transport(&self, packet_count: usize, packet_size: usize) {
        let tx = self.transport_tx.as_ref().expect("No transport_tx in World");
        let packet = vec![0u8; packet_size];
        for _ in 0..packet_count {
            tx.send(bytes::Bytes::from(packet.clone()))
                .expect("Failed to send packet to transport");
        }
        // Small delay to allow async propagation (Stream -> Stats -> Chip)
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    /// BDD Step: Then radio stats should match expected values
    pub fn then_radio_stats_should_match(&self, expected_tx: u64, expected_rx: u64) {
        let stats = self
            .last_radio_stats
            .as_ref()
            .expect("No radio stats fetched. Call when_fetch_radio_stats() first.");

        let chip_id = self.current_chip_id.expect("No current chip set in World").0;

        // Find stats for our chip
        let s = stats
            .iter()
            .find(|s| s.id == chip_id)
            .unwrap_or_else(|| panic!("No stats found for chip {}", chip_id));

        assert_eq!(s.tx_bytes, expected_tx, "Tx bytes mismatch for chip {}", chip_id);
        assert_eq!(s.rx_bytes, expected_rx, "Rx bytes mismatch for chip {}", chip_id);
    }

    /// Helper to setup mock for verifying radio stats
    pub fn given_radio_stats(&self, device_id: u32, tx: u64, rx: u64) {
        let mut stats_vec = self.radio_stats.lock().unwrap();
        let mut stats = netsim_model::stats::NetsimRadioStats::default();
        stats.id = device_id;
        stats.tx_bytes = tx;
        stats.rx_bytes = rx;

        // Replace or Append
        if let Some(existing) = stats_vec.iter_mut().find(|s| s.id == device_id) {
            *existing = stats;
        } else {
            stats_vec.push(stats);
        }
    }

    /// Actual implementation of when_get_radio_stats assuming client support
    pub async fn when_fetch_radio_stats(&mut self) {
        match self.client.get_radio_stats().await {
            Ok(stats) => self.last_radio_stats = Some(stats),
            Err(e) => panic!("Failed to get radio stats: {}", e),
        }
    }
}
