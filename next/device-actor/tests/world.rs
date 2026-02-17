use device_actor::{DeviceActor, DeviceClient};
use device_api::api::{DeviceChipCreate, DeviceCreate};
use device_api::{DeviceConfig, DeviceId};
use link_api::MockLinkClient;
use netsim_model::chip::{
    BluetoothUpdate, ChipClient, ChipUpdate, ChipVariantUpdate, MockChipClient, NetworkKind,
    RadioUpdate,
};
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

/// The BDD World for Device Actor tests.
pub struct World {
    pub client: DeviceClient,
    _actor_task: tokio::task::JoinHandle<()>,
}

impl Drop for World {
    fn drop(&mut self) {
        self._actor_task.abort();
    }
}

impl World {
    /// Creates a new World with default mock clients.
    pub async fn new() -> Self {
        let chip_clients = Self::create_default_chip_clients();
        let link_client = Self::create_default_link_client();
        Self::with_clients(chip_clients, link_client).await
    }

    /// Creates a new World with injected custom mock clients.
    pub async fn with_clients(
        chip_clients: HashMap<NetworkKind, Box<dyn ChipClient>>,
        link_client: MockLinkClient,
    ) -> Self {
        Self::with_clients_and_timeout(chip_clients, link_client, None, None).await
    }

    /// Creates a new World with injected custom mock clients and idle timeout.
    pub async fn with_clients_and_timeout(
        chip_clients: HashMap<NetworkKind, Box<dyn ChipClient>>,
        link_client: MockLinkClient,
        startup_timeout: Option<std::time::Duration>,
        idle_timeout: Option<std::time::Duration>,
    ) -> Self {
        let (runner, client) = device_actor::new();
        let actor = DeviceActor::new(
            chip_clients,
            Arc::new(AtomicU32::new(0)),
            None,
            Box::new(link_client),
            startup_timeout,
            idle_timeout,
        );
        let actor_task = tokio::spawn(runner.run(actor));
        World { client, _actor_task: actor_task }
    }

    pub fn create_default_chip_clients() -> HashMap<NetworkKind, Box<dyn ChipClient>> {
        let mut clients: HashMap<NetworkKind, Box<dyn ChipClient>> = HashMap::new();
        // Add default mocks for common chip kinds
        for kind in [NetworkKind::Bluetooth, NetworkKind::Wifi, NetworkKind::Uwb] {
            clients.insert(kind, Box::new(Self::create_default_mock_chip()));
        }
        clients
    }

    fn create_default_mock_chip() -> MockChipClient {
        let mut mock = MockChipClient::new();
        // netsim_model::chip::Chip is a struct
        mock.expect_read().returning(|_| Ok(netsim_model::chip::Chip::default()));
        mock.expect_update().returning(|_, _| Ok(netsim_model::chip::Chip::default()));
        mock.expect_create().returning(|_| Ok(()));
        mock.expect_delete().returning(|_| Ok(()));
        mock.expect_read_statistics().returning(|| Ok(Box::from([])));
        mock.expect_reset().returning(|_| Ok(()));
        mock.expect_clone_box().returning(|| Box::new(Self::create_default_mock_chip()));
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
            ),
            chip_config: netsim_model::chip::ChipConfig {
                name: chip_name,
                manufacturer: "Netsim".to_string(),
                product_name: "NetsimBeacon".to_string(),
                network_params: netsim_model::chip::NetworkParams::Bluetooth(
                    netsim_model::chip::BluetoothCreate {
                        address: chip_address,
                        bt_properties: Default::default(),
                        mode: netsim_model::chip::BluetoothMode::Device(Default::default()),
                    },
                ),
            },
        }
    }
    /// Creates a Bluetooth ChipUpdate with the specified Low Energy and Classic radio states.
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
}
