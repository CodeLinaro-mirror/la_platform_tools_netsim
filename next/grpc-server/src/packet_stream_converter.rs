// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use bytes::{BufMut, Bytes, BytesMut};
use netsim_proto::{
    common as proto_common,
    hci_packet::{hcipacket::PacketType, HCIPacket},
    packet_streamer::{self, PacketRequest, PacketResponse},
    startup as proto_startup,
};
use packet_stream::{
    error::{PacketStreamError, Result},
    Chip, ChipInfo, ChipKind, DeviceInfo,
};
use protobuf::Enum;

pub fn proto_to_chip_kind(proto: protobuf::EnumOrUnknown<proto_common::ChipKind>) -> ChipKind {
    crate::frontend_converter::from_proto_chip_kind(proto.enum_value_or_default())
        .unwrap_or(ChipKind::UNSPECIFIED)
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
        kind: proto.kind,
        version: proto.version,
        sdk_version: proto.sdk_version,
        build_id: proto.build_id,
        variant: proto.variant,
        arch: proto.arch,
    }
}

pub fn proto_to_chip_info(proto: proto_startup::ChipInfo) -> ChipInfo {
    ChipInfo {
        name: proto.name,
        chip: proto.chip.into_option().map(proto_to_chip),
        device_info: proto.device_info.into_option().map(proto_to_device_info),
    }
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
