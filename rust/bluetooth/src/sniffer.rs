// Copyright 2023-2025 The Android Open Source Project

use crate::utils::ToChipError;
use log::debug;
use netsim_api::chip_error::ChipError;
use netsim_api::chips::{ChipId, SnifferParams};
use netsim_proto::model::Chip as ProtoChip;
use rootcanal::{bluetooth::Bluetooth as Rootcanal, controller::Idc};

/// A stateless struct that provides the behavior for a Bluetooth sniffer.
pub(crate) fn create(
    rootcanal: &Rootcanal,
    chip_id: ChipId,
    _params: &SnifferParams,
) -> Result<(), ChipError> {
    debug!("[{chip_id}] Setting up sniffer chip");
    // Enable scanning on the new controller.
    debug!("[{chip_id}] Enabling scanning");
    let scan_params = vec![0x12, 0x20, 7, 0x01, 0x10, 0x00, 0x10, 0x00, 0x00, 0x00];
    rootcanal.receive_hci(chip_id.as_u32(), Idc::Cmd, &scan_params).to_chip_error()?;
    let scan_enable = vec![0x0c, 0x20, 2, 0x01, 0x00];
    rootcanal.receive_hci(chip_id.as_u32(), Idc::Cmd, &scan_enable).to_chip_error()?;
    Ok(())
}

#[allow(dead_code)]
pub fn update_chip(
    rootcanal: &Rootcanal,
    chip_id: ChipId,
    _old_params: &SnifferParams,
    _new_params: &SnifferParams,
) -> Result<ProtoChip, ChipError> {
    get_chip(rootcanal, chip_id)
}

#[allow(dead_code)]
pub fn get_chip(_rootcanal: &Rootcanal, _chip_id: ChipId) -> Result<ProtoChip, ChipError> {
    Ok(ProtoChip::default())
}
