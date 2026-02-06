use device_api::api::{Chip, DeviceChipCreate};
use device_api::{Device as ApiDevice, Orientation as ApiOrientation, Position as ApiPosition};
use link_api::Link as ApiLink;
use netsim_model::bluetooth::beacon::{AdvertiseData, AdvertiseSettings};
use netsim_model::chip::BleBeacon;
use netsim_model::chip::ChipId;
use netsim_model::chip::ChipKind as ApiChipKind;
use netsim_model::chip::{
    BluetoothCreate, BluetoothMode, BluetoothUpdate, ChipUpdate, ChipVariantUpdate, Radio,
    RadioUpdate,
};
use netsim_proto::common::ChipKind as ProtoChipKind;
use netsim_proto::model::chip::Radio as ProtoRadio;
use netsim_proto::model::ChipCreate;
use netsim_proto::model::Link as ProtoLink;
use netsim_proto::model::PhyKind as ProtoPhyKind;
use netsim_proto::model::{
    Chip as ProtoChip, Device as ProtoDevice, Orientation as ProtoOrientation,
    Position as ProtoPosition,
};
use netsim_proto::protobuf::EnumOrUnknown;
use netsim_proto::protobuf::MessageField;

pub fn to_proto_position(p: ApiPosition) -> ProtoPosition {
    let mut pos = ProtoPosition::new();
    pos.x = p.x;
    pos.y = p.y;
    pos.z = p.z;
    pos
}

pub fn to_proto_orientation(o: ApiOrientation) -> ProtoOrientation {
    let mut orient = ProtoOrientation::new();
    orient.yaw = o.yaw;
    orient.pitch = o.pitch;
    orient.roll = o.roll;
    orient
}

pub fn to_proto_chip_kind(k: ApiChipKind) -> ProtoChipKind {
    match k {
        ApiChipKind::UNSPECIFIED => ProtoChipKind::UNSPECIFIED,
        ApiChipKind::BLUETOOTH => ProtoChipKind::BLUETOOTH,
        ApiChipKind::WIFI => ProtoChipKind::WIFI,
        ApiChipKind::UWB => ProtoChipKind::UWB,
        ApiChipKind::BleBeacon => ProtoChipKind::BLUETOOTH_BEACON,
        // Map unknown/new types to UNSPECIFIED for now
        ApiChipKind::NFC => ProtoChipKind::UNSPECIFIED,
        ApiChipKind::CELLULAR => ProtoChipKind::UNSPECIFIED,
        ApiChipKind::AP => ProtoChipKind::WIFI,
    }
}

pub fn to_proto_chip(c: netsim_model::chip::Chip) -> ProtoChip {
    let mut chip = ProtoChip::new();
    chip.id = c.id;
    chip.kind = EnumOrUnknown::new(to_proto_chip_kind(c.kind));
    chip.name = c.name.unwrap_or_default();
    chip.manufacturer = c.manufacturer.unwrap_or_default();
    chip.product_name = c.product_name.unwrap_or_default();
    // Note: netsim_model Chip has position, but netsim_proto Chip has offset (Position)
    chip.offset = MessageField::some(to_proto_position(c.position));

    if let Some(variant) = c.variant {
        match variant {
            netsim_model::chip::ChipVariant::Bluetooth(bt_model) => {
                let mut bt = netsim_proto::model::chip::Bluetooth::new();
                bt.low_energy = MessageField::some(to_proto_radio(&bt_model.low_energy));
                bt.classic = MessageField::some(to_proto_radio(&bt_model.classic));
                chip.chip = Some(netsim_proto::model::chip::Chip::Bt(bt));
            }
            netsim_model::chip::ChipVariant::Wifi(radio) => {
                chip.chip = Some(netsim_proto::model::chip::Chip::Wifi(to_proto_radio(&radio)));
            }
            netsim_model::chip::ChipVariant::Uwb(radio) => {
                chip.chip = Some(netsim_proto::model::chip::Chip::Uwb(to_proto_radio(&radio)));
            }
            netsim_model::chip::ChipVariant::Cell(_) => {
                // TODO: Add Cell support to proto if available
            }
            netsim_model::chip::ChipVariant::Ap(_) => {
                let mut radio = ProtoRadio::new();
                radio.state = Some(true); //  AP radio is active by default
                chip.chip = Some(netsim_proto::model::chip::Chip::Wifi(radio));
            }
        }
    }

    chip
}

fn to_proto_radio(r: &netsim_model::chip::Radio) -> netsim_proto::model::chip::Radio {
    let mut radio = netsim_proto::model::chip::Radio::new();
    radio.state = Some(r.state.unwrap_or(true));
    radio.range = r.range;
    radio.tx_count = r.tx_count;
    radio.rx_count = r.rx_count;
    radio
}

pub fn to_proto_device(d: ApiDevice) -> ProtoDevice {
    let mut device = ProtoDevice::new();
    device.id = d.id;
    device.name = d.name;
    device.visible = Some(d.visible);
    device.position = MessageField::some(to_proto_position(d.position));
    device.orientation = MessageField::some(to_proto_orientation(d.orientation));
    for chip in d.chips {
        device.chips.push(to_proto_chip(chip));
    }
    device
}

pub fn from_proto_chip_create(c: ChipCreate) -> Option<DeviceChipCreate> {
    // Currently only supports BLE Beacon
    if c.kind.enum_value_or_default() == ProtoChipKind::BLUETOOTH_BEACON || c.has_ble_beacon() {
        let beacon_create = c.ble_beacon();
        let settings = beacon_create.settings.as_ref().map(from_proto_advertise_settings);
        let adv_data = beacon_create.adv_data.as_ref().map(from_proto_advertise_data);
        let scan_response = beacon_create.scan_response.as_ref().map(from_proto_advertise_data);

        let beacon =
            BleBeacon { address: beacon_create.address.clone(), settings, adv_data, scan_response };

        Some(DeviceChipCreate {
            name: c.name,
            manufacturer: c.manufacturer,
            product_name: c.product_name,
            chip: Chip::Beacon(beacon),
        })
    } else if c.kind.enum_value_or_default() == ProtoChipKind::BLUETOOTH {
        let bt_create = BluetoothCreate {
            address: c.address,
            bt_properties: Default::default(),
            mode: BluetoothMode::Device(Default::default()),
        };
        Some(DeviceChipCreate {
            name: c.name,
            manufacturer: c.manufacturer,
            product_name: c.product_name,
            chip: Chip::Bluetooth(bt_create),
        })
    } else {
        None
    }
}

fn from_proto_advertise_settings(
    s: &netsim_proto::model::chip::ble_beacon::AdvertiseSettings,
) -> AdvertiseSettings {
    use netsim_model::bluetooth::beacon::{AdvertiseMode, AdvertiseTxPower, Interval, TxPower};
    use netsim_proto::model::chip::ble_beacon::advertise_settings::Interval as ProtoInterval;
    use netsim_proto::model::chip::ble_beacon::advertise_settings::Tx_power as ProtoTxPower;

    let interval = match s.interval {
        Some(ProtoInterval::AdvertiseMode(mode)) => match mode.enum_value_or_default() {
            netsim_proto::model::chip::ble_beacon::advertise_settings::AdvertiseMode::LOW_POWER => {
                Some(Interval::AdvertiseMode(AdvertiseMode::LowPower))
            }
            netsim_proto::model::chip::ble_beacon::advertise_settings::AdvertiseMode::BALANCED => {
                Some(Interval::AdvertiseMode(AdvertiseMode::Balanced))
            }
            netsim_proto::model::chip::ble_beacon::advertise_settings::AdvertiseMode::LOW_LATENCY => {
                Some(Interval::AdvertiseMode(AdvertiseMode::LowLatency))
            }
        },
        Some(ProtoInterval::Milliseconds(ms)) => Some(Interval::Milliseconds(ms)),
        None => Some(Interval::AdvertiseMode(AdvertiseMode::LowPower)), // Default
        Some(_) => Some(Interval::AdvertiseMode(AdvertiseMode::LowPower)), // Unknown/Future variant
    };

    let tx_power = match s.tx_power {
        Some(ProtoTxPower::TxPowerLevel(level)) => match level.enum_value_or_default() {
            netsim_proto::model::chip::ble_beacon::advertise_settings::AdvertiseTxPower::ULTRA_LOW => {
                Some(TxPower::TxPowerLevel(AdvertiseTxPower::UltraLow))
            }
            netsim_proto::model::chip::ble_beacon::advertise_settings::AdvertiseTxPower::LOW => {
                Some(TxPower::TxPowerLevel(AdvertiseTxPower::Low))
            }
            netsim_proto::model::chip::ble_beacon::advertise_settings::AdvertiseTxPower::MEDIUM => {
                Some(TxPower::TxPowerLevel(AdvertiseTxPower::Medium))
            }
            netsim_proto::model::chip::ble_beacon::advertise_settings::AdvertiseTxPower::HIGH => {
                Some(TxPower::TxPowerLevel(AdvertiseTxPower::High))
            }
        },
        Some(ProtoTxPower::Dbm(dbm)) => Some(TxPower::Dbm(dbm)),
        None => Some(TxPower::TxPowerLevel(AdvertiseTxPower::Low)), // Default
        Some(_) => Some(TxPower::TxPowerLevel(AdvertiseTxPower::Low)), // Unknown/Future variant
    };

    AdvertiseSettings { interval, tx_power, scannable: s.scannable, timeout: s.timeout }
}

fn from_proto_advertise_data(
    d: &netsim_proto::model::chip::ble_beacon::AdvertiseData,
) -> AdvertiseData {
    AdvertiseData {
        include_device_name: d.include_device_name,
        include_tx_power_level: d.include_tx_power_level,
        manufacturer_data: d.manufacturer_data.clone(),
        services: d
            .services
            .iter()
            .map(|s| netsim_model::bluetooth::beacon::Service {
                uuid: s.uuid.clone(),
                data: s.data.clone(),
            })
            .collect(),
    }
}

pub fn from_proto_position(p: ProtoPosition) -> ApiPosition {
    ApiPosition { x: p.x, y: p.y, z: p.z }
}

pub fn from_proto_orientation(o: ProtoOrientation) -> ApiOrientation {
    ApiOrientation { yaw: o.yaw, pitch: o.pitch, roll: o.roll }
}

pub fn from_proto_chip_kind(k: ProtoChipKind) -> ApiChipKind {
    match k {
        ProtoChipKind::UNSPECIFIED => ApiChipKind::UNSPECIFIED,
        ProtoChipKind::BLUETOOTH => ApiChipKind::BLUETOOTH,
        ProtoChipKind::WIFI => ApiChipKind::WIFI,
        ProtoChipKind::UWB => ApiChipKind::UWB,
        ProtoChipKind::BLUETOOTH_BEACON => ApiChipKind::BleBeacon,
    }
}

pub fn to_proto_link(l: ApiLink) -> ProtoLink {
    let mut link = ProtoLink::new();
    link.id = l.id.0;
    link.sender_id = l.sender.0;
    link.receiver_id = l.receiver.0;
    link.rssi = l.rssi as i32;
    link.kind = EnumOrUnknown::new(to_proto_chip_kind(l.kind));
    link.link_kind = EnumOrUnknown::new(match l.kind {
        ApiChipKind::BLUETOOTH => ProtoPhyKind::BLUETOOTH_LOW_ENERGY,
        ApiChipKind::WIFI => ProtoPhyKind::WIFI,
        ApiChipKind::UWB => ProtoPhyKind::UWB,
        ApiChipKind::AP => ProtoPhyKind::WIFI,
        _ => ProtoPhyKind::NONE,
    });
    link
}

pub fn from_proto_chip_update(c: ProtoChip) -> ChipUpdate {
    let variant = if let Some(chip_oneof) = c.chip {
        match chip_oneof {
            netsim_proto::model::chip::Chip::Bt(bt) => {
                Some(ChipVariantUpdate::Bluetooth(BluetoothUpdate {
                    classic: from_proto_radio_update(bt.classic.into_option()),
                    low_energy: from_proto_radio_update(bt.low_energy.into_option()),
                }))
            }
            netsim_proto::model::chip::Chip::BleBeacon(_) => None, // TODO
            netsim_proto::model::chip::Chip::Uwb(uwb) => {
                Some(ChipVariantUpdate::Uwb(from_proto_radio_update(Some(uwb))))
            }
            netsim_proto::model::chip::Chip::Wifi(wifi) => {
                Some(ChipVariantUpdate::Wifi(from_proto_radio_update(Some(wifi))))
            }
            _ => None,
        }
    } else {
        None
    };

    ChipUpdate {
        id: if c.id != 0 { Some(ChipId(c.id)) } else { None },
        name: if c.name.is_empty() { None } else { Some(c.name) },
        manufacturer: if c.manufacturer.is_empty() { None } else { Some(c.manufacturer) },
        product_name: if c.product_name.is_empty() { None } else { Some(c.product_name) },
        position: c.offset.into_option().map(from_proto_position),
        orientation: None, // Proto Chip does not have orientation.
        variant,
        links: None, // TODO
        enabled: None,
    }
}

pub fn from_proto_radio(r: netsim_proto::model::chip::Radio) -> Radio {
    Radio { state: r.state, range: r.range, tx_count: r.tx_count, rx_count: r.rx_count }
}

pub fn from_proto_radio_update(r: Option<netsim_proto::model::chip::Radio>) -> RadioUpdate {
    match r {
        Some(r) => RadioUpdate { state: r.state },
        None => RadioUpdate { state: None },
    }
}

pub fn from_proto_link(proto: ProtoLink) -> Option<ApiLink> {
    let sender = netsim_model::chip::ChipId(proto.sender_id);
    let receiver = netsim_model::chip::ChipId(proto.receiver_id);
    let rssi = proto.rssi as i8;

    let kind = if proto.kind.enum_value_or_default() != ProtoChipKind::UNSPECIFIED {
        from_proto_chip_kind(proto.kind.enum_value_or_default())
    } else {
        match proto.link_kind.enum_value_or_default() {
            ProtoPhyKind::BLUETOOTH_CLASSIC | ProtoPhyKind::BLUETOOTH_LOW_ENERGY => {
                ApiChipKind::BLUETOOTH
            }
            ProtoPhyKind::WIFI | ProtoPhyKind::WIFI_RTT => ApiChipKind::WIFI,
            ProtoPhyKind::UWB => ApiChipKind::UWB,
            _ => return None,
        }
    };

    Some(ApiLink { id: netsim_model::link::LinkId(0), sender, receiver, kind, rssi })
}
