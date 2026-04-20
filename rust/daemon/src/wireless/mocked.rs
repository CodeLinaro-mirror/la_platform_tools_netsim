// Copyright 2023 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::devices::chip::ChipIdentifier;
use crate::wireless::{WirelessChip, WirelessChipImpl};

use bytes::Bytes;
use netsim_proto::common::ChipKind as ProtoChipKind;
use netsim_proto::model::Chip as ProtoChip;
use netsim_proto::stats::{netsim_radio_stats, NetsimRadioStats as ProtoRadioStats};
use protobuf::EnumOrUnknown;

/// Parameters for creating Mocked chips
pub struct CreateParams {
    pub chip_kind: ProtoChipKind,
}

/// Mock struct is remained empty.
pub struct Mock {
    chip_kind: ProtoChipKind,
}

impl WirelessChip for Mock {
    fn handle_request(&self, _packet: &Bytes) {}

    fn reset(&self) {}

    fn get(&self) -> ProtoChip {
        let mut proto_chip = ProtoChip::new();
        proto_chip.kind = EnumOrUnknown::new(self.chip_kind);
        proto_chip
    }

    fn patch(&self, _chip: &ProtoChip) {}

    fn get_stats(&self, _duration_secs: u64) -> Vec<ProtoRadioStats> {
        let mut stats = ProtoRadioStats::new();
        stats.kind = Some(EnumOrUnknown::new(match self.chip_kind {
            ProtoChipKind::BLUETOOTH => netsim_radio_stats::Kind::BLUETOOTH_LOW_ENERGY,
            ProtoChipKind::WIFI => netsim_radio_stats::Kind::WIFI,
            ProtoChipKind::UWB => netsim_radio_stats::Kind::UWB,
            ProtoChipKind::BLUETOOTH_BEACON => netsim_radio_stats::Kind::BLE_BEACON,
            ProtoChipKind::NFC => netsim_radio_stats::Kind::NFC,
            ProtoChipKind::CELLULAR => netsim_radio_stats::Kind::UNSPECIFIED,
            _ => netsim_radio_stats::Kind::UNSPECIFIED,
        }));
        vec![stats]
    }
}

/// Create a new MockedChip
pub fn add_chip(create_params: &CreateParams, _chip_id: ChipIdentifier) -> WirelessChipImpl {
    Box::new(Mock { chip_kind: create_params.chip_kind })
}
