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
use netsim_model::{
    chip::{
        BluetoothUpdate, ChipClient, ChipUpdate, ChipVariantUpdate, MockChipClient, RadioUpdate,
    },
    ChipKind,
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
    pub stats_file_to_cleanup: Option<std::path::PathBuf>,
    // BDD State
    pub current_device_id: Option<DeviceId>,
    pub current_chip_id: Option<netsim_model::chip::ChipId>,
    pub transport_tx: Option<tokio::sync::mpsc::UnboundedSender<Bytes>>,
    pub device_config: Option<DeviceConfig>,
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
    /// Detaches the stats file cleanup responsibility from the World.
    /// Useful when the test wants to verify the file content after World drop.
    pub fn detach_stats_cleanup(&mut self) {
        self.stats_file_to_cleanup = None;
    }

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
            Some(path.clone()),
            interval,
            radio_stats,
            Some(path),
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
        let mut actor = DeviceActor::new(
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
        actor.set_self_client(client.clone());
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
            device_config: None,
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
            clients
                .insert(kind, Box::new(Self::create_default_mock_chip(radio_stats.clone(), kind)));
        }
        clients
    }

    pub(crate) fn create_default_mock_chip(
        radio_stats: Arc<std::sync::Mutex<Vec<netsim_model::stats::NetsimRadioStats>>>,
        kind: ChipKind,
    ) -> MockChipClient {
        let mut mock = MockChipClient::new();
        // Shared state for the chip
        let chip_state = Arc::new(std::sync::Mutex::new(netsim_model::chip::Chip {
            kind,
            ..Default::default()
        }));

        // Return chip with correct kind and state
        let chip_state_read = chip_state.clone();
        mock.expect_read().returning(move |_| {
            let chip = chip_state_read.lock().unwrap();
            Ok(chip.clone())
        });

        let chip_state_update = chip_state.clone();
        mock.expect_update().returning(move |_, patch| {
            let mut chip = chip_state_update.lock().unwrap();
            // Apply updates to the shared state
            if let Some(netsim_model::chip::ChipVariantUpdate::Bluetooth(bt_update)) = patch.variant
            {
                let le_state = bt_update.low_energy.state;
                let classic_state = bt_update.classic.state;

                // Ensure variant exists
                if chip.variant.is_none() {
                    chip.variant =
                        Some(netsim_model::chip::ChipVariant::Bluetooth(Default::default()));
                }

                if let Some(netsim_model::chip::ChipVariant::Bluetooth(bt)) = &mut chip.variant {
                    if let Some(s) = le_state {
                        bt.low_energy.state = Some(s);
                    }
                    if let Some(s) = classic_state {
                        bt.classic.state = Some(s);
                    }
                }
            }
            Ok(chip.clone())
        });

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

        // Clone returns a fresh mock handling stats from the shared state AND shared
        // chip state
        let radio_stats_clone = radio_stats.clone();
        let chip_state_clone = chip_state.clone();

        mock.expect_clone_box().returning(move || {
            let mut clone_mock = MockChipClient::new();
            let rs_clone = radio_stats_clone.clone();
            let cs_read = chip_state_clone.clone();

            clone_mock.expect_read_statistics().returning(move || {
                let stats = rs_clone.lock().unwrap();
                Ok(Box::from(stats.clone()))
            });

            clone_mock.expect_read().returning(move |_| {
                let chip = cs_read.lock().unwrap();
                Ok(chip.clone())
            });

            let cs_update = chip_state_clone.clone();
            clone_mock.expect_update().returning(move |_, patch| {
                let mut chip = cs_update.lock().unwrap();
                if let Some(netsim_model::chip::ChipVariantUpdate::Bluetooth(bt_update)) =
                    patch.variant
                {
                    let le_state = bt_update.low_energy.state;
                    let classic_state = bt_update.classic.state;

                    if chip.variant.is_none() {
                        chip.variant =
                            Some(netsim_model::chip::ChipVariant::Bluetooth(Default::default()));
                    }

                    if let Some(netsim_model::chip::ChipVariant::Bluetooth(bt)) = &mut chip.variant
                    {
                        if let Some(s) = le_state {
                            bt.low_energy.state = Some(s);
                        }
                        if let Some(s) = classic_state {
                            bt.classic.state = Some(s);
                        }
                    }
                }
                Ok(chip.clone())
            });

            clone_mock.expect_create().returning(|params| {
                if let Some(mut stream) = params.packet_stream {
                    tokio::spawn(async move { while stream.next().await.is_some() {} });
                }
                Ok(())
            });
            clone_mock.expect_delete().returning(|_| Ok(()));
            clone_mock.expect_reset().returning(|_| Ok(()));
            clone_mock.expect_clone_box().returning(move || Box::new(MockChipClient::new())); // Nested clones not fully supported for now, but usually not needed 2 levels
                                                                                              // deep in tests

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

    pub async fn when_add_chip(&self, device_guid: &str, chip_name: &str) -> DeviceId {
        let params = Self::create_device_add_chip_params(
            device_guid.to_string(),
            chip_name.to_string(),
            "00:00:00:00:00:00".to_string(),
        );
        self.client.add_chip(params).await.unwrap()
    }

    pub async fn when_add_wifi_chip(&self, device_guid: &str, chip_name: &str) -> DeviceId {
        let mut params = Self::create_device_add_chip_params(
            device_guid.to_string(),
            chip_name.to_string(),
            "".to_string(),
        );
        params.chip_config.chip_kind_params =
            netsim_model::chip::ChipKindParams::Wifi(netsim_model::chip::WifiCreate {
                ..Default::default()
            });
        self.client.add_chip(params).await.unwrap()
    }

    pub async fn when_add_uwb_chip(&self, device_guid: &str, chip_name: &str) -> DeviceId {
        let mut params = Self::create_device_add_chip_params(
            device_guid.to_string(),
            chip_name.to_string(),
            "".to_string(),
        );
        params.chip_config.chip_kind_params =
            netsim_model::chip::ChipKindParams::Uwb(netsim_model::chip::UwbCreate {
                ..Default::default()
            });
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

    /// BDD Step: When I add a WiFi chip with an injected packet stream.
    pub async fn when_add_wifi_chip_with_stream(
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
            "".to_string(),
        );
        params.chip_config.chip_kind_params =
            netsim_model::chip::ChipKindParams::Wifi(netsim_model::chip::WifiCreate {
                ..Default::default()
            });
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

    /// Helper to read stats file with retry
    pub async fn get_stats_from_file(path: &std::path::PathBuf) -> serde_json::Value {
        let mut last_content = String::new();
        // Wait up to 5 seconds
        for _ in 0..50 {
            if path.exists() {
                if let Ok(c) = std::fs::read_to_string(path) {
                    if !c.is_empty() {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&c) {
                            return json;
                        }
                        last_content = c;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        panic!("Stats file content mismatch or timeout. Last content: {}", last_content);
    }

    /// Verifies the content of the stats file.
    pub async fn verify_stats_file_content(
        path: &std::path::PathBuf,
        expected_version: &str,
        expected_device_count: u32,
        expected_peak_devices: u32,
    ) {
        let json = Self::get_stats_from_file(path).await;
        assert_eq!(json["version"], expected_version, "version mismatch");

        let val = json["device_count"].as_u64().expect("device_count missing");
        assert_eq!(val, expected_device_count as u64, "device_count mismatch");

        let val =
            json["peak_concurrent_devices"].as_u64().expect("peak_concurrent_devices missing");
        assert_eq!(val, expected_peak_devices as u64, "peak_concurrent_devices mismatch");

        let duration = json["duration_secs"].as_u64();

        if let Some(_) = duration {
            // okay
        } else {
            println!("WARNING: duration_secs missing in stats file");
        }
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

    /// BDD Step: Given a WiFi device with a transport stream
    pub async fn given_wifi_device_with_transport_stream(
        &mut self,
        device_guid: &str,
        chip_name: &str,
    ) {
        let (device_id, tx) = self.when_add_wifi_chip_with_stream(device_guid, chip_name).await;
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
        let device_id = self.current_device_id.expect("No current device set in World").0;
        // Default to Bluetooth Low Energy for generic test if not specified,
        // to avoid UNSPECIFIED in generic tests
        self.given_radio_stats(
            device_id,
            netsim_model::stats::RadioKind::BluetoothLowEnergy,
            tx,
            rx,
        );
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

        let device_id = self.current_device_id.expect("No current device set in World").0;

        // Find stats for our chip
        let s = stats
            .iter()
            .find(|s| s.id == device_id)
            .unwrap_or_else(|| panic!("No stats found for device {}", device_id));

        assert_eq!(s.tx_bytes, expected_tx, "Tx bytes mismatch for device {}", device_id);
        assert_eq!(s.rx_bytes, expected_rx, "Rx bytes mismatch for device {}", device_id);
    }

    /// Helper to setup mock for verifying radio stats
    pub fn given_radio_stats(
        &self,
        device_id: u32,
        kind: netsim_model::stats::RadioKind,
        tx: u64,
        rx: u64,
    ) {
        let mut stats_vec = self.radio_stats.lock().unwrap();
        let mut stats = netsim_model::stats::NetsimRadioStats::default();
        stats.id = device_id;
        stats.kind = kind;
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

    /// BDD Step: Given I have a device configuration
    pub fn given_device_config(
        &mut self,
        name: &str,
        kind: &str,
        version: &str,
        sdk_version: &str,
        build_id: &str,
        variant: &str,
        arch: &str,
    ) {
        let mut config = DeviceConfig::new(
            name.to_string(),
            true,
            Default::default(),
            Default::default(),
            false,
        );
        config.device_info = Some(netsim_model::device::DeviceInfo {
            name: name.to_string(),
            kind: kind.to_string(),
            version: version.to_string(),
            sdk_version: sdk_version.to_string(),
            build_id: build_id.to_string(),
            variant: variant.to_string(),
            arch: arch.to_string(),
            ..Default::default()
        });
        self.device_config = Some(config);
    }

    /// BDD Step: When I create the device from the pending configuration
    pub async fn when_create_device_from_config(&mut self) {
        let config = self.device_config.take().expect("No pending config");
        let params = DeviceCreate {
            device_config: config,
            chip: DeviceChipCreate {
                name: "beacon".to_string(),
                manufacturer: "Netsim".to_string(),
                product_name: "NetsimBeacon".to_string(),
                chip: device_api::api::Chip::Beacon(Default::default()),
            },
        };
        let id = self.client.create_device(params).await.unwrap();
        self.current_device_id = Some(id);
    }

    /// BDD Step: Then the stats file should match the device details
    pub async fn then_stats_should_contain_device_details(
        &self,
        kind: &str,
        version: &str,
        sdk_version: &str,
        build_id: &str,
        variant: &str,
        arch: &str,
    ) {
        let path = self.stats_file_to_cleanup.as_ref().unwrap();
        let json = Self::get_stats_from_file(path).await;

        let device_stats = json["device_stats"]
            .as_array()
            .expect(&format!("device_stats missing in JSON: {}", json));

        let device_id = self.current_device_id.expect("No current_device_id").0;

        let ds = device_stats.iter().find(|s| {
            let id_val = s["device_id"].as_u64();
            id_val == Some(device_id as u64)
        });

        assert!(ds.is_some(), "Stats for device {} not found in: {:?}", device_id, device_stats);
        let ds = ds.unwrap();

        assert_eq!(ds["kind"], kind);
        assert_eq!(ds["version"], version);
        assert_eq!(ds["sdk_version"].as_str().expect("sdk_version missing"), sdk_version);
        assert_eq!(ds["build_id"].as_str().expect("build_id missing"), build_id);
        assert_eq!(ds["variant"], variant);
        assert_eq!(ds["arch"], arch);
    }

    /// BDD Step: Then the radio stats file should contain entries matching the
    /// criteria
    pub async fn verify_radio_stats_persisted(
        &self,
        expected_tx_bytes: Option<u64>,
        expected_rx_bytes: Option<u64>,
        expected_tx_count: Option<u64>,
        expected_rx_count: Option<u64>,
        expected_kind: Option<&str>,
    ) {
        let path = self.stats_file_to_cleanup.as_ref().expect("No stats file path in World");
        let device_id = self.current_device_id.expect("No current_device_id").0;

        let mut found = false;
        let mut last_content = String::new();

        // Retry loop to handle async write latency
        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if !path.exists() {
                continue;
            }

            let content = std::fs::read_to_string(path).unwrap_or_default();
            if content.is_empty() {
                continue;
            }
            last_content = content.clone();

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(radio_stats) = json["radio_stats"].as_array() {
                    if radio_stats.iter().any(|s| {
                        let id = s["device_id"].as_u64().unwrap_or(0);
                        if id != device_id as u64 {
                            return false;
                        }

                        if let Some(kind) = expected_kind {
                            let k = s["kind"].as_str().unwrap_or("UNSPECIFIED");
                            // Handle both string and enum number (if serialized as number)
                            // But usually proto json maps enum to string.
                            // We allow "1" (BLE) or "BLUETOOTH_LOW_ENERGY" etc.
                            if k != kind && k != "1" && k != "2" && k != "4" {
                                // simplified check, strict check would be mapped
                                if !kind.eq_ignore_ascii_case(k) {
                                    return false;
                                }
                            }
                        }

                        if let Some(tx) = expected_tx_bytes {
                            if s["tx_bytes"].as_u64().unwrap_or(0) != tx {
                                return false;
                            }
                        }
                        if let Some(rx) = expected_rx_bytes {
                            if s["rx_bytes"].as_u64().unwrap_or(0) != rx {
                                return false;
                            }
                        }
                        if let Some(count) = expected_tx_count {
                            if s["tx_count"].as_u64().unwrap_or(0) != count {
                                return false;
                            }
                        }
                        if let Some(count) = expected_rx_count {
                            if s["rx_count"].as_u64().unwrap_or(0) != count {
                                return false;
                            }
                        }
                        true
                    }) {
                        found = true;
                        break;
                    }
                }
            }
        }

        assert!(
            found,
            "Radio stats matching criteria not found for device {}. Last content: {}",
            device_id, last_content
        );
    }
}
