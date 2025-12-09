//! # Device Actor Action Handlers
//!
//! This module contains the handlers for actions performed on the device actor,
//! such as adding chips and resetting the device.

use crate::context::DeviceContext;
use crate::entity::DeviceEntity;
use crate::error::DeviceError;
use crate::handlers::utils::chip_kind_to_network_kind;
use device_api::{DeviceAction, DeviceActionResult};
use netsim_model::chip::{
    BeaconParams, BluetoothCreate, BluetoothMode, ChipCreate, ChipId, NetworkKind, NetworkParams,
};
use netsim_model::device::api::Chip as ApiChip;
use netsim_model::device::DeviceId;
use std::sync::atomic::Ordering;

pub async fn handle_action(
    entity: &mut DeviceEntity,
    action: DeviceAction,
    ctx: &mut DeviceContext,
) -> Result<DeviceActionResult, DeviceError> {
    match action {
        DeviceAction::Reset => {
            // TODO: Implement device reset logic if needed
            Ok(DeviceActionResult::Success)
        }
        DeviceAction::NotifyChipRemoved(_device_id, chip_id) => {
            entity.device.chips.retain(|c| c.id != chip_id.0);
            // TODO: If entity.device.chips.is_empty(), remove the device itself.
            // This requires a way to trigger a self-delete from within the actor.
            Ok(DeviceActionResult::Success)
        }
        DeviceAction::AddChip { chip_config, packet_stream, packet_sink } => {
            let chip_id = ChipId(ctx.next_chip_id.fetch_add(1, Ordering::SeqCst));

            // 1. Create Chip parameters
            let network_params = match &chip_config.chip {
                ApiChip::Beacon(beacon) => NetworkParams::Bluetooth(BluetoothCreate {
                    address: beacon.address.clone(),
                    bt_properties: Default::default(),
                    mode: BluetoothMode::Beacon(Box::new(BeaconParams {
                        ble_beacon: beacon.clone(),
                    })),
                }),
            };
            let chip_kind = NetworkKind::from(&network_params);

            // 2. Handle Capture Creation and Stream Wrapping
            let (packet_stream, packet_sink) = if let Some(capture_client) = &ctx.capture_client {
                crate::handlers::utils::create_capture_and_wrap_streams(
                    capture_client.clone(),
                    chip_id,
                    chip_kind,
                    entity.device.name.clone(),
                    packet_stream,
                    packet_sink,
                )
                .await
            } else {
                (packet_stream, packet_sink)
            };

            let chip_params = ChipCreate {
                id: chip_id,
                packet_stream,
                packet_sink,
                config: netsim_model::chip::ChipConfig {
                    name: chip_config.name.clone(),
                    manufacturer: chip_config.manufacturer.clone(),
                    product_name: chip_config.product_name.clone(),
                    network_params,
                },
                device_id: DeviceId(entity.device.id),
            };

            // 2. Send create request to Chip Actor
            if let Some(chip_client) = ctx.chip_clients.get(&chip_kind) {
                chip_client
                    .create(chip_params)
                    .await
                    .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;

                // 3. Update local device state with the new chip
                entity.device.chips.push(netsim_model::chip::Chip {
                    id: chip_id.0,
                    kind: netsim_model::chip::ChipKind::from(chip_kind),
                    name: Some(chip_config.name),
                    manufacturer: Some(chip_config.manufacturer),
                    product_name: Some(chip_config.product_name),
                    position: entity.device.position.clone(),
                    orientation: entity.device.orientation.clone(),
                    device_id: DeviceId(entity.device.id),
                    variant: None,
                });
                Ok(DeviceActionResult::ChipId(chip_id))
            } else {
                Err(DeviceError::ActorCommunicationError(format!(
                    "No chip client for {:?}",
                    chip_kind
                )))
            }
        }
    }
}
