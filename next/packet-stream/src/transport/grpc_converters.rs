use crate::error::{PacketStreamError, Result};
use bytes::{BufMut, Bytes, BytesMut};
use netsim_api::initial_info::{Chip, ChipInfo, ChipKind, DeviceInfo};
use netsim_proto::common as proto_common;
use netsim_proto::hci_packet::hcipacket::PacketType;
use netsim_proto::hci_packet::HCIPacket;
use netsim_proto::packet_streamer::{self, PacketRequest, PacketResponse};
use netsim_proto::startup as proto_startup;
use protobuf::Enum;
use protobuf::Message;

pub(crate) const HCI_PACKET_TYPE: u8 = 0x01;

pub fn proto_to_chip_kind(proto: protobuf::EnumOrUnknown<proto_common::ChipKind>) -> ChipKind {
    match proto.enum_value_or_default() {
        proto_common::ChipKind::UNSPECIFIED => ChipKind::UNSPECIFIED,
        proto_common::ChipKind::BLUETOOTH => ChipKind::BLUETOOTH,
        proto_common::ChipKind::WIFI => ChipKind::WIFI,
        proto_common::ChipKind::UWB => ChipKind::UWB,
        // proto_common::ChipKind::CELLULAR => ChipKind::CELL,
        proto_common::ChipKind::BLUETOOTH_BEACON => ChipKind::UNSPECIFIED, // Or map to a suitable netsim_api::ChipKind
    }
}

pub fn proto_to_chip(proto: proto_startup::Chip) -> Chip {
    Chip {
        kind: proto_to_chip_kind(proto.kind),
        id: proto.id,
        name: "".to_string(), // Not available in proto Chip
        manufacturer: proto.manufacturer,
        product_name: proto.product_name,
    }
}

pub fn proto_to_device_info(proto: proto_startup::DeviceInfo) -> DeviceInfo {
    DeviceInfo {
        name: proto.name,
        id: "".to_string(), // Not available in proto DeviceInfo
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
            let mut bytes = BytesMut::new();
            bytes.put_u8(HCI_PACKET_TYPE);
            let hci_bytes = hci.write_to_bytes()?;
            bytes.put_slice(&hci_bytes);
            Ok(bytes.freeze())
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
            ChipKind::CELL => proto_common::ChipKind::UNSPECIFIED, // Or map to a suitable netsim_api::ChipKind
        }
        .into();
        chip_proto.id = chip.id;
        chip_proto.manufacturer = chip.manufacturer;
        chip_proto.product_name = chip.product_name;
        proto.chip = Some(chip_proto).into();
    }
    if let Some(device_info) = chip_info.device_info {
        let mut device_proto = proto_startup::DeviceInfo::new();
        device_proto.name = device_info.name;
        proto.device_info = Some(device_proto).into();
    }
    proto
}

impl From<protobuf::Error> for PacketStreamError {
    fn from(err: protobuf::Error) -> Self {
        PacketStreamError::InvalidConfig(format!("Protobuf error: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use netsim_proto::hci_packet::HCIPacket;
    use netsim_proto::startup::ChipInfo as ProtoChipInfo;
    use protobuf::Message;

    #[test]
    fn test_hci_request_conversion() {
        let mut hci = HCIPacket::new();
        hci.packet = vec![0x01, 0x02, 0x03, 0x04];
        let mut req = PacketRequest::new();
        req.set_hci_packet(hci.clone());

        let bytes = packet_request_to_bytes(req).unwrap();
        assert_eq!(bytes.len(), hci.packet.len() + 1);
        assert_eq!(bytes[0], hci.packet_type.value() as u8);
        assert_eq!(&bytes[1..], hci.packet.as_slice());

        // Test bytes_to_packet_request with is_bt = true (H4 format)
        let mut h4_bytes = BytesMut::new();
        h4_bytes.put_u8(hci.packet_type.value() as u8);
        h4_bytes.put_slice(&hci.packet);
        let req2 = bytes_to_packet_request(h4_bytes.freeze(), true).unwrap();
        assert!(req2.has_hci_packet());
        assert_eq!(req2.hci_packet(), &hci);
    }

    #[test]
    fn test_hci_response_conversion() {
        let packet_data = vec![0x05, 0x06, 0x07, 0x08];
        let mut hci = HCIPacket::new();
        hci.packet = packet_data.clone();

        // Test packet_response_to_bytes
        let mut res = PacketResponse::new();
        res.set_hci_packet(hci.clone());
        let bytes_with_prefix = packet_response_to_bytes(res).unwrap();
        assert_eq!(bytes_with_prefix[0], HCI_PACKET_TYPE);
        assert_eq!(bytes_with_prefix.len(), hci.write_to_bytes().unwrap().len() + 1);

        // Test bytes_to_packet_response with is_bt = true
        let mut h4_packet = BytesMut::new();
        h4_packet.put_u8(PacketType::EVENT.value() as u8);
        h4_packet.put_slice(&packet_data);
        let res2 = bytes_to_packet_response(h4_packet.freeze(), true).unwrap();
        assert!(res2.has_hci_packet());
        let expected_hci = HCIPacket {
            packet_type: PacketType::EVENT.into(),
            packet: packet_data.clone(),
            ..Default::default()
        };
        assert_eq!(res2.hci_packet(), &expected_hci);

        // Test bytes_to_packet_response with is_bt = false
        let res3 = bytes_to_packet_response(Bytes::from(packet_data.clone()), false).unwrap();
        assert!(res3.has_packet());
        assert_eq!(res3.packet(), packet_data.as_slice());
    }

    #[test]
    fn test_raw_packet_request() {
        let raw = Bytes::from_static(&[0xAA, 0xBB, 0xCC]);
        let mut req = PacketRequest::new();
        req.set_packet(raw.to_vec());
        let bytes = packet_request_to_bytes(req).unwrap();
        assert_eq!(bytes, raw); // No type byte prepended

        let req2 = bytes_to_packet_request(bytes, false).unwrap();
        assert!(req2.has_packet());
        assert_eq!(req2.packet(), raw.as_ref());
        assert!(!req2.has_hci_packet());
    }

    #[test]
    fn test_raw_packet_response() {
        let raw = Bytes::from_static(&[0xDD, 0xEE, 0xFF]);
        let mut res = PacketResponse::new();
        res.set_packet(raw.to_vec());
        let bytes = packet_response_to_bytes(res).unwrap();
        assert_eq!(bytes, raw); // No type byte prepended

        let res2 = bytes_to_packet_response(bytes, false).unwrap();
        assert!(res2.has_packet());
        assert_eq!(res2.packet(), raw.as_ref());
        assert!(!res2.has_hci_packet());
    }

    #[test]
    fn test_packet_request_to_bytes_error() {
        let mut req = PacketRequest::new();
        req.set_initial_info(ProtoChipInfo::new());
        let result = packet_request_to_bytes(req);
        assert!(result.is_err());
    }

    #[test]
    fn test_packet_response_to_bytes_error() {
        let mut res = PacketResponse::new();
        res.set_error("Test Error".to_string());
        let result = packet_response_to_bytes(res);
        assert!(result.is_err());
    }
}
