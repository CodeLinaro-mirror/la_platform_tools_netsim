// Copyright 2025 The Android Open Source Project

use std::{path::PathBuf, sync::Arc, time::Duration};

use daemon::netsimd::{NetsimDaemon, StartUpMode};
use grpcio::{ChannelBuilder, EnvBuilder};
use netsim_proto::{
    common::ChipKind,
    frontend::{CreateDeviceRequest, DeleteChipRequest},
    frontend_grpc::FrontendServiceClient,
    model::{ChipCreate, DeviceCreate},
    packet_streamer_grpc::PacketStreamerClient,
    protobuf::{EnumOrUnknown, MessageField},
};

/// The BDD World for Daemon tests.
pub struct World {
    pub daemon: Option<NetsimDaemon>,
    pub frontend_client: Option<FrontendServiceClient>,
    pub packet_client: Option<PacketStreamerClient>,
    pub capture_client: capture_actor::CaptureClient,

    pub grpc_port: u16,
    _temp_dir: PathBuf,
    _ini_guard: Option<daemon::ini_file::IniFileGuard>,
}

impl Drop for World {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self._temp_dir);
    }
}

impl World {
    /// Given a running Netsim Daemon
    pub async fn new() -> Self {
        let mut args = daemon::args::Args::default();
        args.logtostderr = true; // Disable log redirection
        args.no_shutdown = true; // Prevent tests from dying when deleting devices
        Self::new_with_args(args).await
    }

    pub async fn new_with_args(args: daemon::args::Args) -> Self {
        let temp_dir = std::env::temp_dir().join(format!("netsim_test_{}", rand::random::<u32>()));
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        let startup_mode = NetsimDaemon::new_with_dirs(temp_dir.clone(), args)
            .await
            .expect("Failed to create daemon");

        let (daemon, ini_guard) = match startup_mode {
            StartUpMode::Owner(daemon, ini_guard) => (daemon, ini_guard),
            _ => panic!("Expected to start as Owner"),
        };

        let grpc_port = daemon.grpc_port().expect("NetsimDaemon has no gRPC port");

        // Allow some time for bindings
        tokio::time::sleep(Duration::from_millis(100)).await;

        let capture_client = daemon.capture_client().clone();

        World {
            daemon: Some(daemon),
            frontend_client: None,
            packet_client: None,
            capture_client,
            grpc_port,
            _temp_dir: temp_dir,
            _ini_guard: Some(ini_guard),
        }
    }

    /// Helper to get or create frontend client
    pub fn ensure_frontend_client(&mut self) -> &FrontendServiceClient {
        if self.frontend_client.is_none() {
            let env = Arc::new(EnvBuilder::new().build());
            let ch = ChannelBuilder::new(env).connect(&format!("localhost:{}", self.grpc_port));
            self.frontend_client = Some(FrontendServiceClient::new(ch));
        }
        self.frontend_client.as_ref().unwrap()
    }

    /// Helper to get or create packet streamer client
    pub fn ensure_packet_client(&mut self) -> &PacketStreamerClient {
        if self.packet_client.is_none() {
            let env = Arc::new(EnvBuilder::new().build());
            let ch = ChannelBuilder::new(env).connect(&format!("localhost:{}", self.grpc_port));
            self.packet_client = Some(PacketStreamerClient::new(ch));
        }
        self.packet_client.as_ref().unwrap()
    }

    /// When I request the version
    pub async fn when_get_version(&mut self) -> String {
        let client = self.ensure_frontend_client();
        let resp = client
            .get_version_async(&netsim_proto::empty::Empty::new())
            .expect("GetVersion failed")
            .await
            .expect("RPC failed");
        resp.version
    }

    /// When I create a device with name and chip
    pub async fn when_create_device(&mut self, name: &str, chip_name: &str) -> u32 {
        let client = self.ensure_frontend_client();
        let mut create_req = CreateDeviceRequest::new();
        let mut device_create = DeviceCreate::new();
        device_create.name = name.to_string();

        let mut chip_create = ChipCreate::new();
        chip_create.name = chip_name.to_string();
        chip_create.kind = EnumOrUnknown::new(ChipKind::BLUETOOTH);
        chip_create.manufacturer = "TestMfg".to_string();
        chip_create.product_name = "TestProduct".to_string();
        chip_create.address = "11:22:33:44:55:66".to_string();
        let mut ble_beacon = netsim_proto::model::chip_create::BleBeaconCreate::new();
        ble_beacon.address = "11:22:33:44:55:66".to_string();
        chip_create.set_ble_beacon(ble_beacon);

        device_create.chips.push(chip_create);
        create_req.device = MessageField::some(device_create);

        let resp = client
            .create_device_async(&create_req)
            .expect("CreateDevice failed")
            .await
            .expect("RPC failed");
        resp.device.id
    }

    /// When I list devices
    pub async fn when_list_devices(&mut self) -> Vec<netsim_proto::model::Device> {
        let client = self.ensure_frontend_client();
        let resp = client
            .list_device_async(&netsim_proto::empty::Empty::new())
            .expect("ListDevice failed")
            .await
            .expect("RPC failed");
        resp.devices
    }

    /// When I delete a chip (device)
    pub async fn when_delete_chip(&mut self, device_id: u32) {
        let client = self.ensure_frontend_client();
        let mut req = DeleteChipRequest::new();
        req.id = device_id;
        client.delete_chip_async(&req).expect("DeleteChip failed").await.expect("RPC failed");
    }

    /// When I create a device with specific chips
    pub async fn when_create_device_with_chips(
        &mut self,
        name: &str,
        chips: Vec<ChipCreate>,
    ) -> u32 {
        let client = self.ensure_frontend_client();
        let mut create_req = CreateDeviceRequest::new();
        let mut device_create = DeviceCreate::new();
        device_create.name = name.to_string();
        device_create.chips = chips;
        create_req.device = MessageField::some(device_create);

        let resp = client
            .create_device_async(&create_req)
            .expect("CreateDevice failed")
            .await
            .expect("RPC failed");
        resp.device.id
    }

    /// When I patch a device
    pub async fn when_patch_device(
        &mut self,
        patch_req: &netsim_proto::frontend::PatchDeviceRequest,
    ) {
        let client = self.ensure_frontend_client();
        client
            .patch_device_async(patch_req)
            .expect("PatchDevice failed")
            .await
            .expect("RPC failed");
    }

    /// Start the daemon in the background (World owns the logical flow, usually
    /// we spawn daemon in a thread or task) Note: In these tests, we often
    /// spawn the daemon task.
    pub fn spawn_daemon(&mut self) -> tokio::task::JoinHandle<()> {
        if let Some(daemon) = self.daemon.take() {
            tokio::spawn(async move {
                let _ = daemon.run_daemon().await;
            })
        } else {
            panic!("Daemon already running or not initialized");
        }
    }

    /// Helper to create a Bluetooth chip configuration
    pub fn make_bluetooth_chip(name: &str, address: &str) -> ChipCreate {
        let mut chip = ChipCreate::new();
        chip.name = name.to_string();
        chip.kind = EnumOrUnknown::new(ChipKind::BLUETOOTH);
        chip.address = address.to_string();
        chip
    }

    /// Helper to create a UWB chip configuration
    pub fn make_uwb_chip(name: &str) -> ChipCreate {
        let mut chip = ChipCreate::new();
        chip.name = name.to_string();
        chip.kind = EnumOrUnknown::new(ChipKind::UWB);
        chip
    }

    /// When I patch the radio state of a chip
    pub async fn when_patch_state(
        &mut self,
        device_id: u32,
        chip_id: Option<u32>,
        kind: ChipKind,
        state: bool,
    ) {
        let mut patch_req = netsim_proto::frontend::PatchDeviceRequest::new();
        patch_req.id = Some(device_id);

        let mut patch_fields =
            netsim_proto::frontend::patch_device_request::PatchDeviceFields::new();

        let mut chip_patch = netsim_proto::model::Chip::new();
        if let Some(id) = chip_id {
            chip_patch.id = id;
        }
        chip_patch.kind = EnumOrUnknown::new(kind);

        if kind == ChipKind::BLUETOOTH {
            let mut bt_data = netsim_proto::model::chip::Bluetooth::new();
            let mut classic = netsim_proto::model::chip::Radio::new();
            classic.state = Some(state);
            bt_data.classic = MessageField::some(classic);
            chip_patch.set_bt(bt_data);
        } else if kind == ChipKind::UWB {
            let mut radio = netsim_proto::model::chip::Radio::new();
            radio.state = Some(state);
            chip_patch.set_uwb(radio);
        }

        patch_fields.chips.push(chip_patch);
        patch_req.device = MessageField::some(patch_fields);

        self.when_patch_device(&patch_req).await;
    }

    /// Then I verify the radio state of a chip
    pub async fn then_radio_state_is(&mut self, device_id: u32, kind: ChipKind, expected: bool) {
        let devices = self.when_list_devices().await;
        let device = devices.iter().find(|d| d.id == device_id).expect("Device missing");

        // Find the chips of the given kind
        let chips: Vec<_> =
            device.chips.iter().filter(|c| c.kind == EnumOrUnknown::new(kind)).collect();

        assert!(!chips.is_empty(), "No chips of kind {:?} found", kind);

        for chip in chips {
            let state = match kind {
                ChipKind::BLUETOOTH => {
                    if let Some(netsim_proto::model::chip::Chip::Bt(bt)) = &chip.chip {
                        bt.classic.as_ref().and_then(|r| r.state).unwrap_or(true)
                    } else {
                        panic!("Chip kind mismatch");
                    }
                }
                ChipKind::UWB => {
                    if let Some(netsim_proto::model::chip::Chip::Uwb(radio)) = &chip.chip {
                        radio.state.unwrap_or(true)
                    } else {
                        panic!("Chip kind mismatch");
                    }
                }
                _ => true, // Default to true for other kinds for now
            };
            assert_eq!(state, expected, "Radio state for chip {} should be {}", chip.id, expected);
        }
    }

    /// When I patch the capture state of a chip
    pub async fn when_patch_capture(&self, chip_id: u32, enabled: bool) {
        self.capture_client
            .patch_capture(netsim_model::chip::ChipId::from(chip_id), enabled)
            .await
            .expect("Failed to patch capture");
    }

    /// Then I verify the capture state of a chip
    pub async fn then_capture_is(&self, chip_id: u32, expected: bool) {
        let captures = self.capture_client.list_captures().await.expect("Failed to list captures");
        let capture = captures
            .iter()
            .find(|c| c.chip_id == netsim_model::chip::ChipId::from(chip_id))
            .expect("Capture info missing");

        assert_eq!(
            capture.enabled, expected,
            "Capture expected for chip {} should be {}",
            chip_id, expected
        );
    }
}
