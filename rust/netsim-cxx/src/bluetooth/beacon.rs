// Copyright 2023 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::advertise_data::{AdvertiseData, AdvertiseDataBuilder};
use super::advertise_settings::{
    AdvertiseMode, AdvertiseSettings, AdvertiseSettingsBuilder, TxPowerLevel,
};
use super::chip::{rust_bluetooth_add, RustBluetoothChipCallbacks};
use crate::devices::chip::{ChipIdentifier, FacadeIdentifier};
use crate::devices::device::{AddChipResult, DeviceIdentifier};
use crate::devices::{devices_handler::add_chip, id_factory::IdFactory};
use crate::ffi::{generate_advertising_packet, generate_scan_response_packet, RustBluetoothChip};
use cxx::{let_cxx_string, UniquePtr};
use frontend_proto::common::ChipKind;
use frontend_proto::model::chip::{
    bluetooth_beacon::AdvertiseData as AdvertiseDataProto,
    bluetooth_beacon::AdvertiseSettings as AdvertiseSettingsProto,
    BluetoothBeacon as BluetoothBeaconProto,
};
use frontend_proto::model::chip_create::{
    BluetoothBeaconCreate as BluetoothBeaconCreateProto, Chip as BuiltinProto,
};
use frontend_proto::model::{ChipCreate as ChipCreateProto, DeviceCreate as DeviceCreateProto};
use lazy_static::lazy_static;
use log::{error, info, warn};
use protobuf::MessageField;
use std::alloc::System;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};
use std::{collections::HashMap, ptr::null};

// PhyType::LOW_ENERGY defined in packages/modules/Bluetooth/tools/rootcanal/include/phy.h
static PHY_TYPE_LE: u8 = 0;

lazy_static! {
    // A singleton that contains a hash map from chip id to RustBluetoothChip.
    // It's used by `BeaconChip` to access `RustBluetoothChip` to call send_link_layer_packet().
    static ref BT_CHIPS: RwLock<HashMap<ChipIdentifier, Mutex<UniquePtr<RustBluetoothChip>>>> =
        RwLock::new(HashMap::new());
    // Used to find beacon chip based on it's id from static methods.
    pub(crate) static ref BEACON_CHIPS: RwLock<HashMap<ChipIdentifier, Mutex<BeaconChip>>> =
        RwLock::new(HashMap::new());
}

/// BeaconChip class.
pub struct BeaconChip {
    chip_id: ChipIdentifier,
    address: String,
    advertise_settings: AdvertiseSettings,
    advertise_data: AdvertiseData,
    advertise_last: Option<Instant>,
}

impl BeaconChip {
    pub fn new(chip_id: ChipIdentifier, address: String) -> Self {
        BeaconChip {
            chip_id,
            address,
            advertise_settings: AdvertiseSettings::builder().build(),
            advertise_data: AdvertiseData::builder().build().unwrap(),
            advertise_last: None,
        }
    }

    pub fn from_proto(
        device_name: String,
        chip_id: ChipIdentifier,
        beacon_proto: &BluetoothBeaconCreateProto,
    ) -> Result<Self, String> {
        let advertise_settings =
            AdvertiseSettingsBuilder::from_proto(&beacon_proto.settings)?.build();
        let advertise_data = AdvertiseDataBuilder::from_proto(
            device_name,
            beacon_proto
                .settings
                .tx_power
                .as_ref()
                .map(TxPowerLevel::try_from)
                .transpose()?
                .unwrap_or_default(),
            &beacon_proto.adv_data,
        )
        .build()?;

        Ok(BeaconChip {
            chip_id,
            address: beacon_proto.address.clone(),
            advertise_settings,
            advertise_data,
            advertise_last: None,
        })
    }

    pub fn send_link_layer_packet(&mut self, packet: &[u8], packet_type: u8, tx_power: i8) {
        let binding = BT_CHIPS.read().unwrap();
        if let Some(rust_bluetooth_chip) = binding.get(&self.chip_id) {
            rust_bluetooth_chip.lock().unwrap().pin_mut().send_link_layer_packet(
                packet,
                packet_type,
                tx_power,
            );
        } else {
            warn!("Failed to get RustBluetoothChip for unknown chip id {}", self.chip_id);
        };
    }
}

// BEACON_CHIPS has ownership of all the BeaconChips, so we need a separate class to hold the callbacks.
// This class will be owned by rootcanal.
pub struct BeaconChipCallbacks {
    chip_id: ChipIdentifier,
}

impl RustBluetoothChipCallbacks for BeaconChipCallbacks {
    fn tick(&mut self) {
        let guard = BEACON_CHIPS.read().unwrap();
        let mut beacon = guard
            .get(&self.chip_id)
            .unwrap_or_else(|| {
                panic!("could not find bluetooth beacon with chip id {}", self.chip_id)
            })
            .lock()
            .unwrap();

        if let Some(last) = beacon.advertise_last {
            if last.elapsed() <= beacon.advertise_settings.mode.interval {
                return;
            }
        }

        beacon.advertise_last = Some(Instant::now());
        let packet = generate_advertising_packet(&beacon.address, beacon.advertise_data.as_bytes());
        beacon.send_link_layer_packet(&packet, PHY_TYPE_LE, TxPowerLevel::default().dbm);
    }

    fn receive_link_layer_packet(
        &mut self,
        source_address: String,
        destination_address: String,
        packet_type: u8,
        packet: &[u8],
    ) {
        // TODO(jmes): Implement by following the example of Beacon::ReceiveLinkLayerPacket()
        //             in packages/modules/Bluetooth/tools/rootcanal/model/devices/beacon.cc.
    }
}

/// Add a beacon device in rootcanal.
///
/// Called by `devices/chip.rs`.
///
/// Similar to `bluetooth_add()`.
#[cfg(not(test))]
pub fn bluetooth_beacon_add(
    device_id: DeviceIdentifier,
    device_name: String,
    chip_id: ChipIdentifier,
    chip_proto: &ChipCreateProto,
) -> Result<FacadeIdentifier, String> {
    let beacon_proto = match &chip_proto.chip {
        Some(BuiltinProto::BleBeacon(beacon_proto)) => beacon_proto,
        _ => return Err(String::from("failed to create ble beacon: unexpected chip type")),
    };

    let beacon_chip = BeaconChip::from_proto(device_name, chip_id, beacon_proto)?;
    if BEACON_CHIPS.write().unwrap().insert(chip_id, Mutex::new(beacon_chip)).is_some() {
        return Err(format!(
            "failed to create a bluetooth beacon chip with id {chip_id}: chip id already exists.",
        ));
    }

    let callbacks: Box<dyn RustBluetoothChipCallbacks> = Box::new(BeaconChipCallbacks { chip_id });
    let add_rust_device_result = rust_bluetooth_add(
        device_id,
        callbacks,
        String::from("beacon"),
        beacon_proto.address.clone(),
    );
    let rust_chip = add_rust_device_result.rust_chip;
    let facade_id = add_rust_device_result.facade_id;
    info!("Creating HCI facade {} for device {} chip {}", facade_id, device_id, chip_id);
    BT_CHIPS.write().unwrap().insert(chip_id, Mutex::new(rust_chip));

    Ok(facade_id)
}

// TODO(jmes) Support removing Beacon (b/292234625).

pub fn bluetooth_beacon_patch(
    chip_id: ChipIdentifier,
    patch: &BluetoothBeaconProto,
) -> Result<(), String> {
    let guard = BEACON_CHIPS.read().unwrap();
    let mut beacon = guard
        .get(&chip_id)
        .ok_or(format!("could not find bluetooth beacon with chip id {chip_id} for patching"))?
        .lock()
        .unwrap();

    // TODO(jmes): Support patching other beacon parameters
    beacon.address = patch.address.clone();
    beacon.advertise_settings.mode =
        patch.settings.interval.as_ref().map(AdvertiseMode::from).unwrap_or_default();

    Ok(())
}

pub fn bluetooth_beacon_get(chip_id: ChipIdentifier) -> Result<BluetoothBeaconProto, String> {
    let guard = BEACON_CHIPS.read().unwrap();
    let beacon = guard
        .get(&chip_id)
        .ok_or(format!("could not get bluetooth beacon with chip id {chip_id}"))?
        .lock()
        .unwrap();

    Ok(BluetoothBeaconProto {
        address: beacon.address.clone(),
        settings: MessageField::some((&beacon.advertise_settings).try_into()?),
        adv_data: MessageField::none(),
        ..Default::default()
    })
}

#[cfg(test)]
pub mod tests {
    use std::thread;

    use frontend_proto::model::chip::bluetooth_beacon::AdvertiseData as AdvertiseDataProto;

    use super::*;
    use crate::bluetooth::{bluetooth_beacon_add, refresh_resource};

    lazy_static! {
        static ref TEST_GUID_GENERATOR: Mutex<IdFactory<u32>> = Mutex::new(IdFactory::new(0, 1));
    }

    fn new_test_beacon_with_interval(interval: Duration) -> Result<DeviceIdentifier, String> {
        let id = TEST_GUID_GENERATOR.lock().unwrap().next_id();

        let add_result = bluetooth_beacon_add(
            0,
            format!("test-device-{:?}", thread::current().id()),
            id,
            &ChipCreateProto {
                name: format!("test-beacon-chip-{:?}", thread::current().id()),
                chip: Some(BuiltinProto::BleBeacon(BluetoothBeaconCreateProto {
                    address: String::from("00:00:00:00:00:00"),
                    settings: MessageField::some(AdvertiseSettingsProto {
                        interval: Some(AdvertiseMode::new(interval).try_into()?),
                        ..Default::default()
                    }),
                    ..Default::default()
                })),
                ..Default::default()
            },
        );
        assert!(add_result.is_ok(), "{}", add_result.unwrap_err());

        Ok(id)
    }

    fn cleanup_beacon(chip_id: ChipIdentifier) {
        BEACON_CHIPS.write().unwrap().remove(&chip_id);
    }

    #[test]
    fn test_beacon_get() {
        let interval = Duration::from_millis(9999);

        let id = new_test_beacon_with_interval(interval).unwrap();

        let beacon = bluetooth_beacon_get(id);
        assert!(beacon.is_ok(), "{}", beacon.unwrap_err());
        let beacon = beacon.unwrap();

        let interval_after_get =
            beacon.settings.interval.as_ref().map(AdvertiseMode::from).unwrap().interval;

        assert_eq!(interval, interval_after_get);
        cleanup_beacon(id);
    }

    #[test]
    fn test_beacon_patch() {
        let interval = Duration::from_millis(33);
        let id = new_test_beacon_with_interval(Duration::from_millis(0)).unwrap();

        let patch_result = bluetooth_beacon_patch(
            id,
            &BluetoothBeaconProto {
                settings: MessageField::some(AdvertiseSettingsProto {
                    interval: Some(AdvertiseMode::new(interval).try_into().unwrap()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        );
        assert!(patch_result.is_ok(), "{}", patch_result.unwrap_err());

        let beacon_proto = bluetooth_beacon_get(id);
        assert!(beacon_proto.is_ok(), "{}", beacon_proto.unwrap_err());
        let interval_after_patch = beacon_proto
            .unwrap()
            .settings
            .interval
            .as_ref()
            .map(AdvertiseMode::from)
            .unwrap()
            .interval;

        assert_eq!(interval, interval_after_patch);
        cleanup_beacon(id);
    }
}
