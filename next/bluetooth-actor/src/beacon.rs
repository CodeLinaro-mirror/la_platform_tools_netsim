// Copyright 2023-2025 The Android Open Source Project

//! This module provides the functionality for creating and managing Bluetooth
//! beacon chips.
//!
//! It includes the `create` function for initializing a new beacon with
//! advertising parameters and data, and placeholder functions for updating
//! and retrieving chip information.
//!
//! NOTE: This module is currently missing the complete setup for converting
//! `BeaconParams` into the appropriate HCI commands for full configuration.

use netsim_model::{
    chip::{BeaconParams, ChipId},
    chip_error::ChipError,
};
use netsim_packets::hci;
use netsim_proto::{hci_packet::hcipacket::PacketType, protobuf::Enum};
use rootcanal::{Address, Rootcanal};
use zerocopy::{Immutable, IntoBytes, KnownLayout, U16};

use crate::{beacon_utils::construct_data, utils::ToChipError};

/// Send an HCI command to the controller.
fn send_hci_command<T: hci::HciCommand + IntoBytes + Immutable + KnownLayout>(
    rootcanal: &Rootcanal,
    chip_id: ChipId,
    payload: T,
) -> Result<(), ChipError> {
    let header = hci::HciCommandHeader {
        op_code: T::OP_CODE,
        parameter_total_length: payload.as_bytes().len() as u8,
    };
    let h4_packet = std::iter::once(PacketType::COMMAND.value() as u8)
        .chain(header.as_bytes().into_iter().copied())
        .chain(payload.as_bytes().into_iter().copied())
        .collect();
    rootcanal.receive_hci(chip_id.into(), h4_packet).to_chip_error()
}

/// Creates a new `BeaconChip`.
pub fn create(
    rootcanal: &Rootcanal,
    chip_id: ChipId,
    params: &BeaconParams,
    device_name: &String,
) -> Result<(), ChipError> {
    // Reset the controller first.
    send_hci_command(rootcanal, chip_id, hci::Reset {})?;

    let address: Address = params.ble_beacon.address.parse().map_err(|_| {
        ChipError::InvalidArguments(Box::from(format!(
            "Invalid beacon address: {}",
            params.ble_beacon.address
        )))
    })?;

    // LE Set Advertising Parameters
    send_hci_command(
        rootcanal,
        chip_id,
        hci::LeSetAdvertisingParameters {
            advertising_interval_min: U16::new(0x00A0),
            advertising_interval_max: U16::new(0x00A0),
            advertising_type: hci::AdvertisingType::ADV_IND,
            own_address_type: hci::OwnAddressType::PUBLIC_DEVICE_ADDRESS,
            peer_address_type: hci::PeerAddressType::PUBLIC_DEVICE_OR_IDENTITY_ADDRESS,
            peer_address: hci::Address { bytes: address.address },
            advertising_channel_map: 0x07,
            advertising_filter_policy: hci::AdvertisingFilterPolicy::ALL_DEVICES,
        },
    )?;

    let adv_data = if let Some(adv_data) = &params.ble_beacon.adv_data {
        construct_data(
            adv_data,
            &if adv_data.include_device_name { Some(device_name.clone()) } else { None },
        )
    } else {
        construct_data(&netsim_model::bluetooth::beacon::AdvertiseData::default(), &None)
    };

    let mut adv_data_payload = [0u8; 31];
    adv_data_payload[..adv_data.len()].copy_from_slice(&adv_data);

    send_hci_command(
        rootcanal,
        chip_id,
        hci::LeSetAdvertisingData {
            advertising_data_length: adv_data.len() as u8,
            advertising_data: adv_data_payload,
        },
    )?;

    // LE Set Scan Response Data
    let scan_resp_data = if let Some(scan_resp) = &params.ble_beacon.scan_response {
        construct_data(
            scan_resp,
            &if scan_resp.include_device_name { Some(device_name.clone()) } else { None },
        )
    } else {
        vec![]
    };

    let mut scan_resp_payload = [0u8; 31];
    scan_resp_payload[..scan_resp_data.len()].copy_from_slice(&scan_resp_data);

    send_hci_command(
        rootcanal,
        chip_id,
        hci::LeSetScanResponseData {
            scan_response_data_length: scan_resp_data.len() as u8,
            scan_response_data: scan_resp_payload,
        },
    )?;

    // LE Set Advertising Enable
    send_hci_command(
        rootcanal,
        chip_id,
        hci::LeSetAdvertisingEnable { advertising_enable: hci::Enable::ENABLED },
    )?;

    Ok(())
}
