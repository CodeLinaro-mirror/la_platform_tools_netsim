use crate::context::DeviceContext;
use crate::entity::DeviceEntity;
use crate::error::DeviceError;
use crate::handlers::utils::chip_kind_to_network_kind;
use device_api::api::Chip as ApiChip;
use device_api::DeviceId;
use netsim_model::chip::{
    BeaconParams, BluetoothCreate, BluetoothMode, ChipConfig, ChipCreate, ChipId, NetworkKind,
    NetworkParams,
};
use std::sync::atomic::Ordering;

pub async fn on_create(
    entity: &mut DeviceEntity,
    ctx: &mut DeviceContext,
) -> Result<(), DeviceError> {
    if let Some(params) = entity.create_params.take() {
        let chip_id = ChipId(ctx.next_chip_id.fetch_add(1, Ordering::SeqCst));
        let chip_create = params.chip;

        // 1. Create Chip parameters
        let network_params = match &chip_create.chip {
            ApiChip::Beacon(beacon) => NetworkParams::Bluetooth(BluetoothCreate {
                address: beacon.address.clone(),
                bt_properties: Default::default(),
                mode: BluetoothMode::Beacon(Box::new(BeaconParams { ble_beacon: beacon.clone() })),
            }),
        };
        let chip_kind = NetworkKind::from(&network_params);

        let chip_params = ChipCreate {
            id: chip_id,
            packet_stream: None,
            packet_sink: None,
            config: ChipConfig {
                name: chip_create.name.clone(),
                manufacturer: chip_create.manufacturer.clone(),
                product_name: chip_create.product_name.clone(),
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
                name: Some(chip_create.name),
                manufacturer: Some(chip_create.manufacturer),
                product_name: Some(chip_create.product_name),
                position: entity.device.position.clone(),
                orientation: entity.device.orientation.clone(),
                device_id: DeviceId(entity.device.id),
                variant: None,
            });
            //TODO: Send create request to Link Actor
        } else {
            // Log warning or return error if no client for this network kind
            return Err(DeviceError::ActorCommunicationError(format!(
                "No chip client for {:?}",
                chip_kind
            )));
        }
    }
    Ok(())
}

pub async fn on_update(
    entity: &mut DeviceEntity,
    update: device_api::api::DeviceUpdate,
    ctx: &mut DeviceContext,
) -> Result<(), DeviceError> {
    // Update local state
    if let Some(name) = update.name {
        entity.device.name = name;
    }
    if let Some(visible) = update.visible {
        entity.device.visible = visible;
    }
    //TODO: check if chip_id is valid
    if let Some(pos) = update.position {
        entity.device.position = pos;
    }
    if let Some(orient) = update.orientation {
        entity.device.orientation = orient;
    }

    // Propagate updates to chips
    for chip in &entity.device.chips {
        let network_kind = chip_kind_to_network_kind(&chip.kind);
        if let Some(chip_client) = ctx.chip_clients.get(&network_kind) {
            let mut chip_update = netsim_model::chip::ChipUpdate::default();
            chip_update.position = Some(entity.device.position.clone());
            chip_update.orientation = Some(entity.device.orientation.clone());

            chip_client
                .update(netsim_model::chip::ChipId(chip.id), chip_update)
                .await
                .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;
            //TODO: overwrite chip links if update.links is Some
        }
    }
    Ok(())
}

pub async fn on_delete(entity: &DeviceEntity, ctx: &mut DeviceContext) -> Result<(), DeviceError> {
    for chip in &entity.device.chips {
        let network_kind = chip_kind_to_network_kind(&chip.kind);
        if let Some(chip_client) = ctx.chip_clients.get(&network_kind) {
            chip_client
                .delete(netsim_model::chip::ChipId(chip.id))
                .await
                .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;
            //TODO: Send delete request to Link Actor
        }
    }
    Ok(())
}

pub fn on_list(
    entities: &std::collections::HashMap<DeviceId, DeviceEntity>,
) -> device_api::api::ListDeviceResponse {
    device_api::api::ListDeviceResponse {
        devices: entities.values().map(|e| e.device.clone()).collect(),
    }
}
