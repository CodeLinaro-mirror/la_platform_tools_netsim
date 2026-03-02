use bytes::{BufMut, Bytes, BytesMut};
use netsim_model::initial_info::{Chip, ChipInfo, ChipKind, DeviceInfo};
use netsim_proto::{
    common as proto_common,
    hci_packet::{hcipacket::PacketType, HCIPacket},
    packet_streamer::{self, PacketRequest, PacketResponse},
    startup as proto_startup,
};
use packet_stream::error::{PacketStreamError, Result};
use protobuf::{Enum, Message};

pub fn proto_to_chip_kind(proto: protobuf::EnumOrUnknown<proto_common::ChipKind>) -> ChipKind {
    match proto.enum_value_or_default() {
        proto_common::ChipKind::UNSPECIFIED => ChipKind::UNSPECIFIED,
        proto_common::ChipKind::BLUETOOTH => ChipKind::BLUETOOTH,
        proto_common::ChipKind::WIFI => ChipKind::WIFI,
        proto_common::ChipKind::UWB => ChipKind::UWB,
        proto_common::ChipKind::BLUETOOTH_BEACON => ChipKind::BLUETOOTH,
    }
}

pub fn proto_to_chip(proto: proto_startup::Chip) -> Chip {
    Chip {
        kind: proto_to_chip_kind(proto.kind),
        id: proto.id,
        name: String::new(), // Not available in proto Chip
        manufacturer: proto.manufacturer,
        product_name: proto.product_name,
        address: proto.address,
    }
}

pub fn proto_to_device_info(proto: proto_startup::DeviceInfo) -> DeviceInfo {
    DeviceInfo {
        name: proto.name,
        id: String::new(), // Not available in proto DeviceInfo
        avd_path: proto.avd_path,
    }
}

pub fn proto_to_chip_info(proto: proto_startup::ChipInfo) -> ChipInfo {
    ChipInfo {
        name: proto.name,
        chip: proto.chip.into_option().map(proto_to_chip),
        device_info: proto.device_info.into_option().map(proto_to_device_info),
    }
}

// Convert Bytes to PacketRequest
pub fn bytes_to_packet_request(bytes: Bytes, is_bt: bool) -> Result<PacketRequest> {
    if bytes.is_empty() {
        return Err(PacketStreamError::InvalidConfig("Empty bytes".to_string()));
    }
    let mut req = PacketRequest::new();
    if is_bt {
        // For Bluetooth, the incoming bytes are H4: IDC + payload
        if let Some(packet_type) = PacketType::from_i32(bytes[0].into()) {
            let hci_packet = HCIPacket {
                packet_type: packet_type.into(),
                packet: bytes.slice(1..).to_vec(),
                ..Default::default()
            };
            req.set_hci_packet(hci_packet);
        } else {
            return Err(PacketStreamError::InvalidConfig(format!(
                "Invalid HCI Packet Type byte: {}",
                bytes[0]
            )));
        }
    } else {
        // For other types, treat as raw packet
        req.set_packet(bytes.to_vec());
    }
    Ok(req)
}

// Convert Bytes to PacketResponse
pub fn bytes_to_packet_response(bytes: Bytes, is_bt: bool) -> Result<PacketResponse> {
    if bytes.is_empty() {
        return Err(PacketStreamError::InvalidConfig("Empty bytes".to_string()));
    }
    let mut res = PacketResponse::new();
    if is_bt {
        let hci_packet = HCIPacket {
            packet_type: PacketType::from_i32(bytes[0].into()).unwrap().into(),
            packet: bytes.slice(1..).to_vec(),
            ..Default::default()
        };
        res.set_hci_packet(hci_packet);
    } else {
        // Assume raw packet if type byte doesn't match
        res.set_packet(bytes.to_vec());
    }
    Ok(res)
}

// Convert PacketRequest to Bytes
pub fn packet_request_to_bytes(value: PacketRequest) -> Result<Bytes> {
    match value.request_type {
        Some(packet_streamer::packet_request::Request_type::HciPacket(hci)) => {
            let mut h4_packet = BytesMut::new();
            h4_packet.put_u8(hci.packet_type.value() as u8);
            let hci_bytes = hci.packet.to_vec();
            h4_packet.put_slice(&hci_bytes);
            Ok(h4_packet.freeze())
        }
        Some(packet_streamer::packet_request::Request_type::Packet(packet)) => {
            Ok(Bytes::from(packet))
        }
        _ => Err(PacketStreamError::InvalidConfig(
            "PacketRequest does not contain a supported packet type".to_string(),
        )),
    }
}

// Convert PacketResponse to Bytes
pub fn packet_response_to_bytes(value: PacketResponse) -> Result<Bytes> {
    match value.response_type {
        Some(packet_streamer::packet_response::Response_type::HciPacket(hci)) => {
            let hci_bytes = hci.write_to_bytes().map_err(|e| {
                PacketStreamError::Protocol(packet_stream::error::ProtocolError::InvalidFormat(
                    e.to_string(),
                ))
            })?;
            Ok(std::iter::once(hci.packet_type.value() as u8).chain(hci_bytes).collect())
        }
        Some(packet_streamer::packet_response::Response_type::Packet(packet)) => {
            Ok(Bytes::from(packet))
        }
        _ => Err(PacketStreamError::InvalidConfig(
            "PacketResponse does not contain a supported packet type".to_string(),
        )),
    }
}

// Helper to convert ChipInfo to proto
pub fn chip_info_to_proto(chip_info: ChipInfo) -> proto_startup::ChipInfo {
    let mut proto = proto_startup::ChipInfo::new();
    proto.name = chip_info.name;
    if let Some(chip) = chip_info.chip {
        let mut chip_proto = proto_startup::Chip::new();
        chip_proto.kind = match chip.kind {
            ChipKind::UNSPECIFIED => proto_common::ChipKind::UNSPECIFIED,
            ChipKind::BLUETOOTH => proto_common::ChipKind::BLUETOOTH,
            ChipKind::WIFI => proto_common::ChipKind::WIFI,
            ChipKind::UWB => proto_common::ChipKind::UWB,
            ChipKind::CELL => proto_common::ChipKind::UNSPECIFIED,
            ChipKind::AP => proto_common::ChipKind::UNSPECIFIED,
        }
        .into();
        chip_proto.id = chip.id;
        chip_proto.manufacturer = chip.manufacturer;
        chip_proto.product_name = chip.product_name;
        chip_proto.address = chip.address;
        proto.chip = Some(chip_proto).into();
    }
    if let Some(device_info) = chip_info.device_info {
        let mut device_proto = proto_startup::DeviceInfo::new();
        device_proto.name = device_info.name;
        device_proto.avd_path = device_info.avd_path;
        proto.device_info = Some(device_proto).into();
    }
    proto
}
