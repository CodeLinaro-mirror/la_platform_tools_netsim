use super::DeviceClient;
use crate::DeviceError;
use device_api::{DeviceAction, DeviceActionResult};
use log::debug;
use netsim_model::device::api::DeviceCreate;
use netsim_model::device::DeviceId;

impl DeviceClient {
    /// Creates or updates a device based on PacketStream parameters.
    ///
    /// This method handles the logic for PacketStream-based device creation, which
    /// includes checking for existing devices by GUID to support multi-chip devices.
    ///
    /// # How it works
    /// 1. Checks if a device with the given `device_guid` already exists in the local state.
    /// 2. If it exists, it adds a new chip to that device using `DeviceAction::AddChip`.
    /// 3. If it doesn't exist, it creates a new device and stores the mapping from GUID to the new Device ID.
    ///
    /// This ensures that multiple PacketStream connections with the same GUID are grouped under a single device.
    pub async fn add_chip(
        &self,
        params: netsim_model::device::DeviceAddChip,
    ) -> Result<DeviceId, DeviceError> {
        debug!("Processing add_chip for GUID {}", params.device_guid);

        let existing_id = {
            let state = self
                .state
                .lock()
                .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;
            state.guid_to_id.get(&params.device_guid).copied()
        };

        if let Some(device_id) = existing_id {
            // If device already existed, add a new chip
            debug!("Adding chip to existing device {}", device_id);
            match self
                .inner
                .perform_action(
                    Some(device_id),
                    DeviceAction::AddChip {
                        chip_config: convert_chip_config(&params.chip_config),
                        packet_stream: params.packet_stream,
                        packet_sink: params.packet_sink,
                    },
                )
                .await
            {
                Ok(DeviceActionResult::ChipId(_)) => Ok(device_id),
                Ok(_) => Err(DeviceError::ActorCommunicationError(
                    "Unexpected action result".to_string(),
                )),
                Err(e) => Err(DeviceError::ActorCommunicationError(e.to_string())),
            }
        } else {
            // Create new device
            let create_params = DeviceCreate {
                device_config: params.device_config.clone(),
                chip: convert_chip_config(&params.chip_config),
            };
            let id = self.create_device(create_params).await?;

            let mut state = self
                .state
                .lock()
                .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;
            state.guid_to_id.insert(params.device_guid, id);
            Ok(id)
        }
    }
}

fn convert_chip_config(
    c: &netsim_model::chip::ChipConfig,
) -> netsim_model::device::api::DeviceChipCreate {
    netsim_model::device::api::DeviceChipCreate {
        name: c.name.clone(),
        manufacturer: c.manufacturer.clone(),
        product_name: c.product_name.clone(),
        chip: c.network_params.clone().into(),
    }
}
