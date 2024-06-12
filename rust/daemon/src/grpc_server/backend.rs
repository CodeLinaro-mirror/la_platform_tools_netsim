// Copyright 2024 Google LLC
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

use crate::devices::chip;
use crate::devices::device::AddChipResult;
use crate::devices::devices_handler;
use crate::transport::grpc::RustGrpcTransport;
use crate::wireless;
use crate::wireless::packet::{register_transport, unregister_transport};
use anyhow::{anyhow, Context};
use bytes::Bytes;
use futures_util::{FutureExt as _, TryFutureExt as _, TryStreamExt as _};
use log::{info, warn};
use netsim_proto::common::ChipKind as ProtoChipKind;
use netsim_proto::hci_packet::hcipacket::PacketType;
use netsim_proto::packet_streamer::{PacketRequest, PacketResponse};
use netsim_proto::packet_streamer_grpc::PacketStreamer;
use netsim_proto::startup::ChipInfo;
use protobuf::Enum;

fn add_chip(initial_info: &ChipInfo, device_guid: &str) -> anyhow::Result<AddChipResult> {
    let chip_kind =
        initial_info.chip.kind.enum_value().map_err(|v| anyhow!("unknown chip kind {v}"))?;
    let chip = &initial_info.chip;
    // TODO(b/323899010): Avoid having cfg(test) in mainline code
    #[cfg(not(test))]
    let wireless_create_param = match &chip_kind {
        ProtoChipKind::BLUETOOTH => {
            wireless::CreateParam::Bluetooth(wireless::bluetooth::CreateParams {
                address: chip.address.clone(),
                bt_properties: None,
            })
        }
        ProtoChipKind::WIFI => wireless::CreateParam::Wifi(wireless::wifi::CreateParams {}),
        ProtoChipKind::UWB => wireless::CreateParam::Uwb(wireless::uwb::CreateParams {
            address: chip.address.clone(),
        }),
        _ => return Err(anyhow!("The provided chip kind is unsupported: {:?}", chip.kind)),
    };
    #[cfg(test)]
    let wireless_create_param =
        wireless::CreateParam::Mock(wireless::mocked::CreateParams { chip_kind });

    let chip_create_params = chip::CreateParams {
        kind: chip_kind,
        address: chip.address.clone(),
        name: Some(chip.id.clone()),
        manufacturer: chip.manufacturer.clone(),
        product_name: chip.product_name.clone(),
        bt_properties: None,
    };

    devices_handler::add_chip(
        device_guid,
        &initial_info.name,
        &chip_create_params,
        &wireless_create_param,
    )
    .map_err(|err| anyhow!(err))
}

#[derive(Clone)]
pub struct PacketStreamerService;
impl PacketStreamer for PacketStreamerService {
    fn stream_packets(
        &mut self,
        ctx: ::grpcio::RpcContext,
        mut packet_request: ::grpcio::RequestStream<PacketRequest>,
        sink: ::grpcio::DuplexSink<PacketResponse>,
    ) {
        let peer = ctx.peer().clone();
        let f = async move {
            info!("grpc_server new packet_stream for peer {}", &peer);

            let request = packet_request.try_next().await?.context("initial info")?;
            let initial_info: &ChipInfo = request.initial_info();

            let result = add_chip(initial_info, &peer)?;

            register_transport(result.chip_id, Box::new(RustGrpcTransport { sink }));

            while let Some(request) = packet_request.try_next().await? {
                let chip_kind = &initial_info.chip.kind.unwrap();
                match chip_kind {
                    ProtoChipKind::WIFI | ProtoChipKind::UWB => {
                        if !request.has_packet() {
                            warn!("unknown packet type from chip_id: {}", result.chip_id);
                            continue;
                        }
                        let packet: Bytes = request.packet().to_vec().into();
                        wireless::handle_request(
                            result.chip_id,
                            &packet,
                            PacketType::HCI_PACKET_UNSPECIFIED.value() as u8,
                        );
                    }
                    ProtoChipKind::BLUETOOTH => {
                        if !request.has_hci_packet() {
                            warn!("unknown packet type from chip_id: {}", result.chip_id);
                            continue;
                        }
                        let packet: Bytes = request.hci_packet().packet.to_vec().into();
                        wireless::handle_request(
                            result.chip_id,
                            &packet,
                            request.hci_packet().packet_type.unwrap().value() as u8,
                        );
                    }
                    _ => {
                        warn!("unknown control packet chip_kind: {:?}", chip_kind);
                        break;
                    }
                };
            }
            unregister_transport(result.chip_id);
            if let Err(e) = devices_handler::remove_chip(result.device_id, result.chip_id) {
                warn!("failed to remove chip: {}", e);
            }
            Ok(())
        }
        .map_err(|e: anyhow::Error| warn!("failed to stream packets: {:?}", e))
        .map(|_| ());
        ctx.spawn(f)
    }
}
