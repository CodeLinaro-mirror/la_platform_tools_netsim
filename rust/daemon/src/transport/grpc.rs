// Copyright 2023 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::wireless::packet::Response;
use bytes::Bytes;
use futures_executor::block_on;
use futures_util::SinkExt as _;
use grpcio::WriteFlags;
use netsim_proto::hci_packet::{hcipacket::PacketType, HCIPacket};
use netsim_proto::packet_streamer::PacketResponse;
use protobuf::Enum;
use protobuf::EnumOrUnknown;

/// Grpc transport.
pub struct RustGrpcTransport {
    pub sink: grpcio::DuplexSink<PacketResponse>,
}

impl Response for RustGrpcTransport {
    fn response(&mut self, packet: Bytes, packet_type: u8) {
        let mut response = PacketResponse::new();
        if packet_type != (PacketType::HCI_PACKET_UNSPECIFIED.value() as u8) {
            let hci_packet = HCIPacket {
                packet_type: EnumOrUnknown::from_i32(packet_type as i32),
                packet: packet.to_vec(),
                ..Default::default()
            };
            response.set_hci_packet(hci_packet);
        } else {
            response.set_packet(packet.to_vec());
        }
        block_on(async {
            let _ = self.sink.send((response, WriteFlags::default())).await;
        });
    }
}
