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

use super::advertise_data::{self, AdvertiseData};
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
    BluetoothBeaconCreate as BluetoothBeaconCreateProto, Chip as ChipProto,
};
use frontend_proto::model::{ChipCreate as ChipCreateProto, DeviceCreate as DeviceCreateProto};
use lazy_static::lazy_static;
use log::{error, info, warn};
use protobuf::MessageField;
use std::alloc::System;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};
use std::{collections::HashMap, ptr::null};

// $ROOTCANAL := packages/modules/Bluetooth/tools/rootcanal

// Default parameter value for SendLinkLayerPacket in $ROOTCANAL/model/devices/device.h
static DEFAULT_TX_POWER: i8 = 0;
// PhyType::LOW_ENERGY defined in $ROOTCANAL/include/phy.h
static PHY_TYPE_LE: u8 = 0;
// From Beacon::Beacon constructor referenced in $ROOTCANAL/model/devices/beacon.cc
static ADVERTISING_INTERVAL_MS: u64 = 1280;

lazy_static! {
    // A singleton that contains a hash map from chip id to RustBluetoothChip.
    // It's used by `BeaconChip` to access `RustBluetoothChip` to call send_link_layer_packet().
    static ref BT_CHIPS: RwLock<HashMap<ChipIdentifier, Mutex<UniquePtr<RustBluetoothChip>>>> =
        RwLock::new(HashMap::new());
    // Used to find beacon chip based on it's id from static methods.
    pub(crate) static ref BEACON_CHIPS: RwLock<HashMap<ChipIdentifier, Mutex<BeaconChip>>> =
        RwLock::new(HashMap::new());
    // Factory for beacon device GUIDs.
    static ref BEACON_DEVICE_GUID_FACTORY: Mutex<IdFactory<usize>> = Mutex::new(IdFactory::new(0, 1));
}

/// BeaconChip class.
pub struct BeaconChip {
    chip_id: ChipIdentifier,
    address: String,
    advertise_data: AdvertiseData,
    advertise_last: Option<Instant>,
    advertise_interval: Duration,
}

impl BeaconChip {
    pub fn new(chip_id: ChipIdentifier, address: String) -> Self {
        BeaconChip {
            chip_id,
            address,
            advertise_data: advertise_data::Builder::new().build().unwrap(),
            advertise_last: None,
            advertise_interval: Duration::from_millis(ADVERTISING_INTERVAL_MS),
        }
    }

    pub fn from_proto(
        device_name: String,
        chip_id: ChipIdentifier,
        beacon_proto: &BluetoothBeaconCreateProto,
    ) -> Result<Self, String> {
        Ok(BeaconChip {
            chip_id,
            address: beacon_proto.address.clone(),
            advertise_data: advertise_data::Builder::from_proto(
                device_name,
                beacon_proto
                    .settings
                    .tx_power_level
                    .try_into()
                    .map_err(|_| "tx_power_level was too large, it must fit in an i8")?,
                &beacon_proto.adv_data,
            )
            .build()?,
            advertise_last: None,
            advertise_interval: Duration::from_millis(beacon_proto.settings.interval),
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
            if last.elapsed() <= beacon.advertise_interval {
                return;
            }
        }

        beacon.advertise_last = Some(Instant::now());
        let packet = generate_advertising_packet(&beacon.address, &beacon.advertise_data.bytes);
        beacon.send_link_layer_packet(&packet, PHY_TYPE_LE, DEFAULT_TX_POWER);
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
    chip_id: ChipIdentifier,
    device_type: String,
    address: String,
) -> Result<FacadeIdentifier, String> {
    let beacon_chip = BeaconChip::new(chip_id, address.clone());

    if BEACON_CHIPS.write().unwrap().insert(chip_id, Mutex::new(beacon_chip)).is_some() {
        return Err(format!(
            "Failed to create a Bluetooth beacon chip with ID {chip_id}: chip ID already exists.",
        ));
    }

    let callbacks: Box<dyn RustBluetoothChipCallbacks> = Box::new(BeaconChipCallbacks { chip_id });
    let add_rust_device_result = rust_bluetooth_add(device_id, callbacks, device_type, address);
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
    beacon.advertise_interval = Duration::from_millis(patch.settings.interval);

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
        settings: MessageField::some(AdvertiseSettingsProto {
            interval: beacon
                .advertise_interval
                .as_millis()
                .try_into()
                .map_err(|err| format!("{err}"))?,
            ..Default::default()
        }),
        adv_data: MessageField::none(),
        ..Default::default()
    })
}

/// Create a new beacon device. Used by CLI or web.
pub fn new_beacon(device_proto: &DeviceCreateProto) -> Result<AddChipResult, String> {
    // TODO(jmes): Support passing BluetoothBeacon and call patch_device().

    let chip_proto = match device_proto.chips.as_slice() {
        [chip_proto] => chip_proto,
        _ => return Err(String::from("a beacon device must contain exactly one chip")),
    };

    let (chip_kind, beacon_proto) = match &chip_proto.chip {
        Some(ChipProto::BleBeacon(beacon_proto)) => (ChipKind::BLUETOOTH_BEACON, beacon_proto),
        Some(_) | None => {
            return Err(String::from("a beacon device must contain a bluetooth beacon chip"))
        }
    };

    let mut device_guid = BEACON_DEVICE_GUID_FACTORY.lock().unwrap().next_id();
    let ids = add_chip(
        &format!("beacon-device-{device_guid}"),
        &device_proto.name,
        chip_kind,
        &chip_proto.name,
        &chip_proto.manufacturer,
        &chip_proto.product_name,
    )?;

    bluetooth_beacon_patch(
        ids.chip_id,
        // TODO(jmes, hyunjaemoon) proto cleanup should prevent us from doing this conversion and clone
        &BluetoothBeaconProto {
            address: beacon_proto.address.clone(),
            settings: beacon_proto.settings.clone(),
            adv_data: beacon_proto.adv_data.clone(),
            ..Default::default()
        },
    )?;

    Ok(ids)
}

#[cfg(test)]
pub mod tests {
    use frontend_proto::model::chip::bluetooth_beacon::AdvertiseData as AdvertiseDataProto;

    use super::*;
    use crate::bluetooth::{bluetooth_beacon_add, refresh_resource};

    fn new_test_beacon_with_interval(interval: u64) -> Result<AddChipResult, String> {
        new_beacon(&DeviceCreateProto {
            name: String::from("test-beacon-device"),
            chips: vec![ChipCreateProto {
                name: String::from("test-beacon-chip"),
                chip: Some(ChipProto::BleBeacon(BluetoothBeaconCreateProto {
                    address: String::from("00:00:00:00:00:00"),
                    settings: MessageField::some(AdvertiseSettingsProto {
                        interval,
                        ..Default::default()
                    }),
                    ..Default::default()
                })),
                ..Default::default()
            }],
            ..Default::default()
        })
    }

    fn cleanup_beacon(chip_id: ChipIdentifier) {
        BEACON_CHIPS.write().unwrap().remove(&chip_id);
    }

    #[test]
    fn test_beacon_get() {
        let interval = 9999;

        let ids = new_test_beacon_with_interval(interval);
        assert!(ids.is_ok());
        let chip_id = ids.unwrap().chip_id;

        let beacon = bluetooth_beacon_get(chip_id).unwrap_or_else(|_| {
            panic!("could not get bluetooth beacon with id {chip_id} for testing")
        });

        assert_eq!(interval, beacon.settings.interval);
        cleanup_beacon(chip_id);
    }

    #[test]
    fn test_beacon_patch() {
        let chip_id: ChipIdentifier = 0;
        let interval = 33;

        let ids = new_test_beacon_with_interval(0);
        assert!(ids.is_ok());
        let chip_id = ids.unwrap().chip_id;

        let patch_result = bluetooth_beacon_patch(
            chip_id,
            &BluetoothBeaconProto {
                settings: MessageField::some(AdvertiseSettingsProto {
                    interval,
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        assert!(patch_result.is_ok(), "{}", patch_result.unwrap_err());

        let beacon_proto = bluetooth_beacon_get(chip_id);

        assert!(beacon_proto.is_ok(), "{}", beacon_proto.unwrap_err());
        assert_eq!(interval, beacon_proto.unwrap().settings.interval);
        cleanup_beacon(chip_id);
    }
}
