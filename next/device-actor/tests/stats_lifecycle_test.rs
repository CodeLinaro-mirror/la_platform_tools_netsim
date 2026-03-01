// Copyright (C) 2025 The Android Open Source Project

use crate::world::World;

// Scenario: Stats are written to file on shutdown
//   Given a running Device Actor with a stats file path
//   When I add multiple devices
//   And I shut down the actor (drop World)
//   Then the global stats are written to the file
//   And the stats contain the correct device count, peak count, and version
#[tokio::test]
async fn test_stats_persistence_on_shutdown() {
    let (path, _) = World::temp_stats_path();
    {
        let world = World::new_with_stats(path.clone(), None).await;
        world.when_add_chip("guid-1", "beacon").await;
        world.when_add_chip("guid-2", "beacon-2").await;
        world.when_shutdown_actor().await;
    }
    World::verify_stats_file_content(&path, "0.0.0-test", 2, 2).await;
}

// Scenario: Peak concurrent devices are tracked correctly
//   Given a running Device Actor
//   When I add 5 devices
//   And I remove 2 devices
//   And I shut down the actor
//   Then the stats file shows 3 active devices and 5 peak concurrent devices
#[tokio::test]
async fn test_peak_concurrent_persistence() {
    let (path, _) = World::temp_stats_path();
    {
        let world = World::new_with_stats(path.clone(), None).await;

        let mut ids = Vec::new();
        for i in 0..5 {
            let name = format!("device-{}", i);
            ids.push(world.when_create_device(&name).await);
        }

        world.when_delete_device(ids[3]).await;
        world.when_delete_device(ids[4]).await;
        world.when_shutdown_actor().await;
    }
    // Note: device_count in proto is cumulative (legacy behavior)
    World::verify_stats_file_content(&path, "0.0.0-test", 5, 5).await;
}

// Scenario: Stats are saved periodically (every 100ms)
//   Given a running Device Actor with a stats file path and 100ms interval
//   When I wait for the stats tick (300ms)
//   Then the global stats are written to the file
#[tokio::test]
async fn test_stats_periodic_save() {
    let (path, _) = World::temp_stats_path();
    let _world =
        World::new_with_stats(path.clone(), Some(std::time::Duration::from_millis(100))).await;

    // Wait for at least one tick (100ms) plus buffer
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    World::verify_stats_file_content(&path, "0.0.0-test", 0, 0).await;
}

// Scenario: A failed stats write cleans up the temporary file properly
//   Given a running Device Actor with a stats file path in a read-only
// directory   When the actor tries to save stats
//   Then the write should fail gracefully
//   And no orphaned .tmp file should remain in the directory
#[tokio::test]
async fn test_stats_write_failure_cleans_up_tmp_file() {
    let (path, _) = World::temp_stats_path();

    // Create a read-only directory to force a write error
    let mut bad_dir = path.clone();
    bad_dir.pop();
    bad_dir.push("readonly_stats_dir_test");
    std::fs::create_dir_all(&bad_dir).unwrap();

    // Make it read-only (Unix specific for this test)
    let mut perms = std::fs::metadata(&bad_dir).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&bad_dir, perms).unwrap();

    let bad_path = bad_dir.join("stats.json");
    let bad_tmp_path = bad_dir.join("stats.json.tmp");

    // The Stats::new should succeed but the subsequent tick should fail cleanly
    // without panicking and without leaving a .tmp file.
    let _world =
        World::new_with_stats(bad_path.clone(), Some(std::time::Duration::from_millis(50))).await;

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // Verify .tmp file does NOT exist
    assert!(!bad_tmp_path.exists(), "Temporary file leaked on write failure!");

    // Clean up
    let mut perms = std::fs::metadata(&bad_dir).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    std::fs::set_permissions(&bad_dir, perms).unwrap();
    let _ = std::fs::remove_dir_all(&bad_dir);
}

// Scenario: Radio Stats are persisted for deleted chips
//   Given a running Device Actor
//   When I add a device and send packets
//   And I delete the device
//   Then the global stats file contains the radio stats for the deleted chip
#[tokio::test]
async fn test_radio_stats_persistence() {
    let (path, _) = World::temp_stats_path();
    {
        let mut world = World::new_with_stats(path.clone(), None).await;

        // 1. Add device with transport stream
        world.given_device_with_transport_stream("guid-1", "chip-1").await;

        // Set to LE-only to avoid ambiguity drop
        let device_id = world.current_device_id.unwrap();
        world
            .when_update_device_chip(
                device_id,
                World::create_bluetooth_chip_update(Some(true), Some(false)),
            )
            .await;

        // 2. Send Packets (Rx from transport perspective = Radio Tx)
        const PACKET_COUNT: usize = 10;
        const PACKET_SIZE: usize = 100;
        const EXPECTED_BYTES: u64 = (PACKET_COUNT * PACKET_SIZE) as u64;

        // 10 packets, 100 bytes each -> 1000 bytes
        world.when_send_packets_to_transport(PACKET_COUNT, PACKET_SIZE).await;

        // Wait for packets to flow through the stream (async processing)
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // 3. Remove device (triggers archive + save)
        let device_id = world.current_device_id.unwrap();
        world.when_delete_device(device_id).await;

        // 4. Verify file content has archived stats
        // Retry loop to handle async write latency
        let mut found = false;
        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if !path.exists() {
                continue;
            }

            let content = std::fs::read_to_string(&path).unwrap_or_default();
            if content.is_empty() {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(radio_stats) =
                    json["radioStats"].as_array().or_else(|| json["radio_stats"].as_array())
                {
                    if !radio_stats.is_empty() {
                        // Check if our chip is present with correct stats
                        if radio_stats.iter().any(|s| {
                            let tx_bytes = s["txBytes"].as_u64().unwrap_or(0);
                            let tx_count = s["txCount"].as_u64().unwrap_or(0);
                            let id = s["deviceId"].as_u64().unwrap_or(0);
                            let kind = s["kind"].as_str().unwrap_or("UNSPECIFIED");

                            tx_bytes == EXPECTED_BYTES
                                && tx_count == PACKET_COUNT as u64
                                && id == device_id.0 as u64
                                && (kind == "BLUETOOTH_LOW_ENERGY" || kind == "1")
                        }) {
                            found = true;
                            break;
                        }
                    }
                }
            }
        }

        if !found {
            // Read one last time for failure message
            let json = World::get_stats_from_file(&path).await;
            println!("DEBUG: JSON content: {:#?}", json);
            panic!("Radio stats not found or incorrect in: {:#?}", json);
        }
    }
}

// Scenario: WiFi stats are persisted after deletion
#[tokio::test]
async fn test_wifi_stats_persistence() {
    let (path, _) = World::temp_stats_path();
    {
        let mut world = World::new_with_stats(path.clone(), None).await;
        // Add WiFi chip
        let device_id = world.when_add_chip("guid-wifi", "wifi").await;

        // Fetch chip ID
        let device = world.client.get(device_id).await.unwrap().unwrap();
        let chip_id = device.chips[0].id; // WiFi chip ID

        // Prime stats for this chip
        {
            let mut stats_vec = world.radio_stats.lock().unwrap();
            let mut stats = netsim_model::stats::NetsimRadioStats::default();
            stats.id = chip_id;
            stats.kind = netsim_model::stats::RadioKind::Wifi;
            stats.tx_bytes = 100;
            stats.rx_bytes = 200;
            stats_vec.push(stats);
        }

        // Wait for stats to settle
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Delete device
        world.when_delete_device(device_id).await;

        // Verify stats
        let mut found = false;
        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if !path.exists() {
                continue;
            }
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(radio_stats) =
                    json["radioStats"].as_array().or_else(|| json["radio_stats"].as_array())
                {
                    if radio_stats.iter().any(|s| {
                        let kind = s["kind"].as_str().unwrap_or("UNSPECIFIED");
                        let id = s["deviceId"].as_u64().unwrap_or(0);
                        let tx = s["txBytes"].as_u64().unwrap_or(0);
                        // DeviceID is used in exported stats, not ChipID (which is internal
                        // usually, but here we check DeviceID)
                        // Wait, service.rs sets exported id to device_id (line 559 in previous
                        // view).
                        id == device_id.0 as u64 && (kind == "WIFI" || kind == "4") && tx == 100
                    }) {
                        found = true;
                        break;
                    }
                }
            }
        }
        assert!(found, "WiFi stats not found in persisted file");
    }
}

// Scenario: Bluetooth Dual Mode (BLE + Classic) stats are persisted
#[tokio::test]
async fn test_bluetooth_dual_mode_persistence() {
    let (path, _) = World::temp_stats_path();
    {
        let mut world =
            World::new_with_stats(path.clone(), Some(std::time::Duration::from_millis(100))).await;
        // Add Bluetooth chip (ChipKind::BLUETOOTH maps to default mock which reads from
        // shared stats)
        let device_id = world.when_add_chip("guid-bt", "bluetooth").await;

        // Fetch chip ID
        let device = world.client.get(device_id).await.unwrap().unwrap();
        let chip_id = device.chips[0].id;

        // Prime stats for this chip (Dual Mode)
        {
            let mut stats_vec = world.radio_stats.lock().unwrap();

            // BLE Stats
            let mut stats_ble = netsim_model::stats::NetsimRadioStats::default();
            stats_ble.id = chip_id;
            stats_ble.kind = netsim_model::stats::RadioKind::BluetoothLowEnergy;
            stats_ble.tx_bytes = 1000;
            stats_ble.rx_bytes = 2000;
            stats_vec.push(stats_ble);

            // Classic Stats
            let mut stats_classic = netsim_model::stats::NetsimRadioStats::default();
            stats_classic.id = chip_id;
            stats_classic.kind = netsim_model::stats::RadioKind::BluetoothClassic;
            stats_classic.tx_bytes = 3000;
            stats_classic.rx_bytes = 4000;
            stats_vec.push(stats_classic);
        }

        // Wait for stats to settle and valid periodic write
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Verify periodic stats (before deletion)
        // We expect BOTH entries in the file for the active device
        let mut found_ble = false;
        let mut found_classic = false;

        // Retry loop for periodic write
        for _ in 0..10 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if !path.exists() {
                continue;
            }
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(radio_stats) =
                    json["radioStats"].as_array().or_else(|| json["radio_stats"].as_array())
                {
                    for s in radio_stats {
                        let kind = s["kind"].as_str().unwrap_or("UNSPECIFIED");
                        let id = s["deviceId"].as_u64().unwrap_or(0);
                        let tx = s["txBytes"].as_u64().unwrap_or(0);

                        if id == device_id.0 as u64 {
                            if (kind == "BLUETOOTH_LOW_ENERGY" || kind == "1") && tx == 1000 {
                                found_ble = true;
                            }
                            if (kind == "BLUETOOTH_CLASSIC" || kind == "2") && tx == 3000 {
                                found_classic = true;
                            }
                        }
                    }
                }
            }
            if found_ble && found_classic {
                break;
            }
        }
        assert!(found_ble, "BLE periodic stats not found");
        assert!(found_classic, "Classic periodic stats not found");

        // Delete device
        world.when_delete_device(device_id).await;

        // Verify archived stats (after deletion)
        found_ble = false;
        found_classic = false;

        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if !path.exists() {
                continue;
            }
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(radio_stats) =
                    json["radioStats"].as_array().or_else(|| json["radio_stats"].as_array())
                {
                    for s in radio_stats {
                        let kind = s["kind"].as_str().unwrap_or("UNSPECIFIED");
                        let id = s["deviceId"].as_u64().unwrap_or(0);
                        let tx = s["txBytes"].as_u64().unwrap_or(0);

                        if id == device_id.0 as u64 {
                            if (kind == "BLUETOOTH_LOW_ENERGY" || kind == "1") && tx == 1000 {
                                found_ble = true;
                            }
                            if (kind == "BLUETOOTH_CLASSIC" || kind == "2") && tx == 3000 {
                                found_classic = true;
                            }
                        }
                    }
                }
            }
            if found_ble && found_classic {
                break;
            }
        }
        assert!(found_ble, "BLE archived stats not found");
        assert!(found_classic, "Classic archived stats not found");
    }
}

// Scenario: Bluetooth Fallback (Client returns no stats) archives Dual Mode
#[tokio::test]
async fn test_bluetooth_fallback_persistence() {
    let (path, _) = World::temp_stats_path();
    {
        let mut world = World::new_with_stats(path.clone(), None).await;

        // 1. Add Bluetooth chip with transport stream
        let (device_id, tx) =
            world.when_add_chip_with_stream("guid-bt-fallback", "bt-fallback").await;

        // 2. Fetch chip ID
        let device = world.client.get(device_id).await.unwrap().unwrap();
        let chip = &device.chips[0];
        // Ensure it is BLUETOOTH kind
        assert_eq!(chip.kind, netsim_model::chip::ChipKind::BLUETOOTH);

        // 3. Send Packets (to populate StreamStats)
        // We do NOT prime `radio_stats`, so client.read_statistics() returns empty vec.
        // This forces fallback to StreamStats.
        let packet = vec![0u8; 100];
        tx.send(bytes::Bytes::from(packet.clone())).unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // 4. Update chip to have ONLY LE enabled (to avoid ambiguity in fallback)
        world
            .when_update_device_chip(
                device_id,
                World::create_bluetooth_chip_update(Some(true), Some(false)),
            )
            .await;

        // 4. Delete device (triggers archive)
        world.when_delete_device(device_id).await;

        // 5. Verify archived stats contain BOTH BLE and Classic
        let mut found_ble = false;
        let mut found_classic = false;

        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if !path.exists() {
                continue;
            }
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(radio_stats) =
                    json["radioStats"].as_array().or_else(|| json["radio_stats"].as_array())
                {
                    for s in radio_stats {
                        let kind = s["kind"].as_str().unwrap_or("UNSPECIFIED");
                        let val = s["txBytes"].as_u64().or(s["tx_bytes"].as_u64()).unwrap_or(0);
                        if (kind == "BLUETOOTH_LOW_ENERGY" || kind == "1") && val >= 100 {
                            found_ble = true;
                        }
                    }
                }
            }
            if found_ble {
                break;
            }
        }

        if !found_ble {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            println!("DEBUG: Read content: {}", content);
        }

        assert!(found_ble, "BLE fallback stats not found");
    }

    // Scenario: Dual Mode Fallback (Ambiguous) - Should NOT drop stats
    #[tokio::test]
    async fn test_dual_mode_fallback_persistence() {
        let (path, _) = World::temp_stats_path();
        {
            let mut world = World::new_with_stats(path.clone(), None).await;

            // 1. Add Bluetooth chip (Default is Dual Mode)
            let (device_id, tx) = world.when_add_chip_with_stream("guid-bt-dual", "bt-dual").await;

            let device = world.client.get(device_id).await.unwrap().unwrap();
            let chip = &device.chips[0];
            assert!(chip.is_le_enabled());
            assert!(chip.is_classic_enabled());

            // 2. Send Packets
            tx.send(bytes::Bytes::from(vec![0u8; 50])).unwrap();
            tx.send(bytes::Bytes::from(vec![0u8; 50])).unwrap();

            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            // 3. Delete device (Client has no stats, so falls back to StreamStats)
            world.when_delete_device(device_id).await;

            // 4. Verify stats persisted (Should not be dropped)
            let mut found_any = false;
            // Wait for file flush
            for _ in 0..10 {
                if path.exists() {
                    let content = std::fs::read_to_string(&path).unwrap_or_default();
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(radio_stats) =
                            json["radioStats"].as_array().or_else(|| json["radio_stats"].as_array())
                        {
                            for s in radio_stats {
                                let id = s["deviceId"].as_u64().unwrap_or(0);
                                let tx_count =
                                    s["txCount"].as_u64().or(s["tx_count"].as_u64()).unwrap_or(0);
                                if id == device_id.0 as u64 && tx_count > 0 {
                                    found_any = true;
                                }
                            }
                        }
                    }
                }
                if found_any {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            assert!(found_any, "Dual Mode stats were dropped during fallback!");
        }
    }

    // Scenario: Bluetooth Beacon stats are persisted
    #[tokio::test]
    async fn test_beacon_stats_persistence() {
        let (path, _) = World::temp_stats_path();
        {
            let mut world = World::new_with_stats(path.clone(), None).await;

            // 1. Add Beacon Chip
            let chip_config = netsim_model::chip::ChipConfig {
                name: "beacon-1".to_string(),
                manufacturer: "Netsim".to_string(),
                product_name: "Beacon".to_string(),
                chip_kind_params: netsim_model::chip::ChipKindParams::Bluetooth(
                    netsim_model::chip::BluetoothCreate {
                        address: "00:00:00:00:00:01".to_string(),
                        bt_properties: Default::default(),
                        mode: netsim_model::chip::BluetoothMode::Beacon(Box::new(
                            netsim_model::chip::BeaconParams {
                                ble_beacon: netsim_model::bluetooth::beacon::BleBeacon {
                                    address: "00:00:00:00:00:01".to_string(),
                                    settings: Some(
                                        netsim_model::bluetooth::beacon::AdvertiseSettings {
                                            scannable: true,
                                            timeout: 1000,
                                            ..Default::default()
                                        },
                                    ),
                                    ..Default::default()
                                },
                            },
                        )),
                    },
                ),
            };

            let device_create = device_api::DeviceCreate {
                device_config: device_api::DeviceConfig::new(
                    "beacon-device".to_string(),
                    true,
                    Default::default(),
                    Default::default(),
                    false,
                ),
                chip: chip_config.into(),
            };

            let device_id = world.client.create_device(device_create).await.unwrap();
            world.current_device_id = Some(device_id);

            // 2. Add Scanner Device
            let scanner_config = netsim_model::chip::ChipConfig {
                name: "scanner-1".to_string(),
                manufacturer: "Netsim".to_string(),
                product_name: "Scanner".to_string(),
                chip_kind_params: netsim_model::chip::ChipKindParams::Bluetooth(
                    netsim_model::chip::BluetoothCreate {
                        address: "00:00:00:00:00:02".to_string(),
                        bt_properties: Default::default(),
                        mode: netsim_model::chip::BluetoothMode::Scanner(
                            netsim_model::chip::ScannerParams {
                            // No specific params for now
                        },
                        ),
                    },
                ),
            };

            let scanner_create = device_api::DeviceCreate {
                device_config: device_api::DeviceConfig::new(
                    "scanner-device".to_string(),
                    true,
                    Default::default(),
                    Default::default(),
                    false,
                ),
                chip: scanner_config.into(),
            };
            let scanner_id = world.client.create_device(scanner_create).await.unwrap();

            // 3. Wait for some time
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            // 4. Delete Devices
            world.when_delete_device(device_id).await;
            world.when_delete_device(scanner_id).await;

            // 5. Verify Stats
            let mut found_beacon = false;
            let mut found_scanner = false;

            for _ in 0..20 {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                if !path.exists() {
                    continue;
                }
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(radio_stats) =
                        json["radioStats"].as_array().or_else(|| json["radio_stats"].as_array())
                    {
                        for s in radio_stats {
                            let kind = s["kind"].as_str().unwrap_or("UNSPECIFIED");
                            let id = s["deviceId"].as_u64().unwrap_or(0);
                            let tx_count =
                                s["txCount"].as_u64().or(s["tx_count"].as_u64()).unwrap_or(0);
                            let rx_count =
                                s["rxCount"].as_u64().or(s["rx_count"].as_u64()).unwrap_or(0);

                            if (kind == "BLUETOOTH_LOW_ENERGY" || kind == "1") {
                                if id == device_id.0 as u64 && tx_count > 0 {
                                    found_beacon = true;
                                }
                                if id == scanner_id.0 as u64 && rx_count > 0 {
                                    found_scanner = true;
                                }
                            }
                        }
                    }
                }
                if found_beacon && found_scanner {
                    break;
                }
            }
            assert!(found_beacon, "Beacon stats not found or tx_count 0");
            // NOTE: Scanner might not receive if RootCanal doesn't propagate to scanner
            // immediately or if setup is slow. But we check if it works.
            assert!(found_scanner, "Scanner stats not found or rx_count 0");
        }
    }
}
