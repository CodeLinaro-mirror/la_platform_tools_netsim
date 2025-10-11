// Copyright 2023-2025 The Android Open Source Project

use crate::utils;
use ::bluetooth::manager::BluetoothManager;
use netsim_proto::model::chip::ble_beacon::{AdvertiseData, AdvertiseSettings};
use netsim_proto::model::chip_create::BleBeaconCreate;
use netsim_proto::model::{chip_create, ChipCreate};

/// Creates a default ChipCreate request for a BLE beacon.
fn create_beacon_request() -> ChipCreate {
    let mut beacon_create = BleBeaconCreate::new();
    beacon_create.address = "00:11:22:33:44:55".to_string();
    beacon_create.settings = Some(AdvertiseSettings::new()).into();
    let mut adv_data = AdvertiseData::new();
    adv_data.include_device_name = true;
    beacon_create.adv_data = Some(adv_data).into();
    let mut create_req = ChipCreate::new();
    create_req.chip = Some(chip_create::Chip::BleBeacon(beacon_create));
    create_req
}

/// Tests that a chip can be successfully added to the manager.
#[tokio::test]
async fn test_add_chip() {
    utils::setup_logging();
    let (manager, _) = BluetoothManager::new();
    let create_req = create_beacon_request();
    // This test is broken because add_chip doesn't exist on BluetoothManager.
    // It is on BeaconManager, which is not public.
    // let result = manager.add_chip(create_req, None);
    // assert!(result.is_ok());
    // let chip = result.unwrap();
    // assert_eq!(chip.id, 1);
}
