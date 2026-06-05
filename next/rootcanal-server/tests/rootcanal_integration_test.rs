// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "cuttlefish")]

use std::{path::PathBuf, time::Duration};

use daemon::{Args, IniFileInitialized, NetsimDaemon, StartUpMode};
use device_actor::DeviceClient;
use netsim_model::{Chip, ChipKind, ChipVariant, DeviceAddChip, DeviceConfig, DeviceId, Pose};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

/// A lightweight, standalone World implementation for testing the gated legacy
/// Rootcanal server. Reuses NetsimDaemon and DeviceClient in-process without
/// gRPC dependencies.
struct RootcanalWorld {
    daemon: Option<NetsimDaemon>,
    daemon_task: Option<tokio::task::JoinHandle<()>>,
    temp_dir: PathBuf,
    test_port: u16,
    _ini_guard: Option<IniFileInitialized>,
    device_client: DeviceClient,
}

impl Drop for RootcanalWorld {
    fn drop(&mut self) {
        if let Some(task) = self.daemon_task.take() {
            task.abort();
        }
        let _ = std::fs::remove_dir_all(&self.temp_dir);
    }
}

impl RootcanalWorld {
    /// Creates a new RootcanalWorld with a daemon configured on dynamic OS
    /// ports.
    async fn new() -> Self {
        let args = Args {
            logtostderr: true,
            no_shutdown: true,
            // Set test_port and hci_port to 0 to bind to dynamic, random OS ports
            test_port: Some(0),
            hci_port: Some(0),
            ..Default::default()
        };

        let temp_dir =
            std::env::temp_dir().join(format!("netsim_rootcanal_test_{}", rand::random::<u32>()));
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        let startup_mode = NetsimDaemon::new_with_dirs(temp_dir.clone(), args)
            .await
            .expect("Failed to create daemon");

        let (daemon, ini_guard) = match startup_mode {
            StartUpMode::Owner(daemon, ini_guard) => (daemon, ini_guard),
            _ => panic!("Expected daemon to start as Owner"),
        };

        // Extract the actual dynamically bound Rootcanal test port
        let test_port = daemon.test_port().expect("Rootcanal control port failed to bind");
        let device_client = daemon.device_client();

        RootcanalWorld {
            daemon: Some(daemon),
            daemon_task: None,
            temp_dir,
            test_port,
            _ini_guard: Some(ini_guard),
            device_client,
        }
    }

    /// Spawns the daemon background event loop task
    async fn when_spawn_daemon(&mut self) {
        if self.daemon_task.is_some() {
            panic!("Daemon already spawned");
        }
        if let Some(daemon) = self.daemon.take() {
            let task = tokio::spawn(async move {
                let _ = daemon.run_daemon().await;
            });
            self.daemon_task = Some(task);
            // Allow time to yield and let the listener bind
            tokio::time::sleep(Duration::from_millis(50)).await;
        } else {
            panic!("Daemon has already been consumed or not initialized");
        }
    }

    /// Connects a local test client to the legacy Rootcanal TCP port
    async fn when_connect_client(&self) -> RootcanalTestClient {
        let stream = TcpStream::connect(("127.0.0.1", self.test_port))
            .await
            .expect("Failed to connect to Rootcanal legacy control TCP port");
        RootcanalTestClient { stream }
    }

    /// Directly provisions a virtual device with a Bluetooth chip via
    /// DeviceClient (bypassing gRPC)
    async fn when_create_device_with_bluetooth(
        &self,
        device_guid: &str,
        chip_address: &str,
    ) -> u32 {
        let mut device_config =
            DeviceConfig::new(device_guid.to_string(), true, Pose::default(), false);
        device_config.device_info = Some(netsim_model::DeviceInfo {
            name: "RootcanalTestDevice".to_string(),
            ..Default::default()
        });

        let add_chip_params = DeviceAddChip {
            device_guid: device_guid.to_string(),
            packet_stream: None,
            packet_sink: None,
            device_config,
            chip: Chip {
                name: format!("chip_{}", device_guid),
                manufacturer: "Netsim".to_string(),
                product_name: "RootcanalBeacon".to_string(),
                kind: ChipKind::BLUETOOTH,
                variant: Some(ChipVariant::Bluetooth(Box::new(netsim_model::Bluetooth {
                    address: chip_address.to_string(),
                    mode: netsim_model::BluetoothMode::Device(Default::default()),
                    bt_properties: Default::default(),
                    ..Default::default()
                }))),
                enabled: true, // Initial state
                ..Default::default()
            },
        };

        let device_id =
            self.device_client.add_chip(add_chip_params).await.expect("Failed to add test chip");
        device_id.0
    }

    /// Asserts whether the Bluetooth chip(s) on a virtual device are
    /// enabled/disabled
    async fn then_bluetooth_enabled_is(&self, device_id: u32, expected: bool) {
        let device =
            self.device_client.get(DeviceId(device_id)).await.unwrap().expect("Device missing");

        let bt_chips: Vec<_> =
            device.chips.iter().filter(|c| c.kind == ChipKind::BLUETOOTH).collect();

        assert!(!bt_chips.is_empty(), "No bluetooth chips found on device {}", device_id);
        for chip in bt_chips {
            assert_eq!(
                chip.enabled, expected,
                "Bluetooth chip {} enabled state mismatch: expected {}, got {}",
                chip.id, expected, chip.enabled
            );
        }
    }
}

/// A simple TCP client that implements the packed binary Rootcanal test channel
/// protocol.
struct RootcanalTestClient {
    stream: TcpStream,
}

impl RootcanalTestClient {
    /// Reads a response from the server (4-byte LE size + UTF-8 string)
    async fn read_response(&mut self) -> String {
        let mut size_buf = [0u8; 4];
        self.stream.read_exact(&mut size_buf).await.unwrap();
        let size = u32::from_le_bytes(size_buf) as usize;

        let mut content_buf = vec![0u8; size];
        self.stream.read_exact(&mut content_buf).await.unwrap();
        String::from_utf8(content_buf).unwrap()
    }

    /// Sends a legacy Rootcanal control command using packed binary
    /// serialization
    async fn send_command(&mut self, name: &str, args: &[&str]) -> String {
        let mut payload = Vec::new();

        payload.push(name.len() as u8);
        payload.extend_from_slice(name.as_bytes());

        payload.push(args.len() as u8);

        for arg in args {
            payload.push(arg.len() as u8);
            payload.extend_from_slice(arg.as_bytes());
        }

        self.stream.write_all(&payload).await.unwrap();
        self.stream.flush().await.unwrap();

        if name != "CLOSE_TEST_CHANNEL" { self.read_response().await } else { "".to_string() }
    }
}

// ======================================================================
// TESTS
// ======================================================================

/// Scenario 1: TCP Connection Handshake
///   Given a running Netsim Daemon
///   When a TCP client connects to the Rootcanal control port
///   Then it immediately receives the welcome handshake string "RootCanal\n"
#[tokio::test]
async fn test_rootcanal_handshake() {
    let mut world = RootcanalWorld::new().await;
    world.when_spawn_daemon().await;

    let mut client = world.when_connect_client().await;

    let welcome = client.read_response().await;
    assert_eq!(welcome, "RootCanal\n", "Welcome handshake mismatch");
}

/// Scenario 2: Listing Virtual Devices and Phys
///   Given a running Netsim Daemon
///   And a virtual device with a Bluetooth chip is created
///   When the TCP client connects and sends the 'list' command
///   Then the response contains the device listed under Devices: and Phys:
/// LOW_ENERGY and BR_EDR categories
#[tokio::test]
async fn test_rootcanal_list_devices() {
    let mut world = RootcanalWorld::new().await;
    let device_id = world.when_create_device_with_bluetooth("guid_1", "11:22:33:44:55:66").await;

    world.when_spawn_daemon().await;

    let mut client = world.when_connect_client().await;
    let _welcome = client.read_response().await; // Discard welcome

    let list_output = client.send_command("list", &[]).await;

    let expected_device_row = format!("{}:hci_device_{}", device_id, device_id);
    assert!(list_output.contains("Devices:"), "Response missing Devices: category");
    assert!(
        list_output.contains(&expected_device_row),
        "Response missing expected device entry: {}",
        expected_device_row
    );
    assert!(list_output.contains("Phys:"), "Response missing Phys: category");

    let le_line = list_output
        .lines()
        .find(|line| line.trim().starts_with("0:LOW_ENERGY:"))
        .expect("Missing LOW_ENERGY physical category");
    let le_devices = le_line.split(':').nth(2).unwrap().trim();
    let le_device_ids: Vec<&str> = le_devices.split(',').map(|s| s.trim()).collect();
    assert!(
        le_device_ids.contains(&device_id.to_string().as_str()),
        "Device {} not found in LOW_ENERGY devices: {:?}",
        device_id,
        le_device_ids
    );

    let classic_line = list_output
        .lines()
        .find(|line| line.trim().starts_with("1:BR_EDR:"))
        .expect("Missing BR_EDR physical category");
    let classic_devices = classic_line.split(':').nth(2).unwrap().trim();
    let classic_device_ids: Vec<&str> = classic_devices.split(',').map(|s| s.trim()).collect();
    assert!(
        classic_device_ids.contains(&device_id.to_string().as_str()),
        "Device {} not found in BR_EDR devices: {:?}",
        device_id,
        classic_device_ids
    );
}

/// Scenario 3: Disconnecting and Reconnecting Device via Phys
///   Given a running Netsim Daemon
///   And a virtual device with a Bluetooth chip is created
///   When the TCP client sends 'del_device_from_phy' for the device
///   Then the Bluetooth chip on the device becomes disabled
///   When the TCP client sends 'add_device_to_phy' for the device
///   Then the Bluetooth chip on the device becomes enabled again
#[tokio::test]
async fn test_rootcanal_phy_toggle() {
    let mut world = RootcanalWorld::new().await;
    let device_id = world.when_create_device_with_bluetooth("guid_2", "11:22:33:44:55:77").await;

    world.when_spawn_daemon().await;

    let mut client = world.when_connect_client().await;
    let _welcome = client.read_response().await; // Discard welcome

    world.then_bluetooth_enabled_is(device_id, true).await;

    // 1. Disconnect from Low Energy Phy
    let dev_id_str = device_id.to_string();
    let resp1 = client.send_command("del_device_from_phy", &[&dev_id_str, "0"]).await;
    assert_eq!(resp1, "OK");

    // 2. Disconnect from Classic Phy
    let resp2 = client.send_command("del_device_from_phy", &[&dev_id_str, "1"]).await;
    assert_eq!(resp2, "OK");

    world.then_bluetooth_enabled_is(device_id, false).await;

    // 3. Connect back to Low Energy Phy
    let resp3 = client.send_command("add_device_to_phy", &[&dev_id_str, "0"]).await;
    assert_eq!(resp3, "OK");

    world.then_bluetooth_enabled_is(device_id, true).await;
}
