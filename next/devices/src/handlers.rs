// Copyright 2023-2025 The Android Post Source Project

use crate::server::{DeviceInfo, Server};
use log::info;
use netsim_api::chips::ChipId;
use netsim_api::{
    chips::{
        BeaconParams, BluetoothMode, BluetoothParams, ChipConfig, ChipPatch,
        CreateParams as ChipCreateParams, NetworkKind, NetworkParams,
    },
    device_error::DeviceError,
    devices::{api, Device, DeviceId, DevicePsCreate, DeviceRequest},
};
use std::collections::HashSet;

impl Server {
    /// This is the main entry point for handling all `DeviceRequest` commands.
    pub(crate) async fn handle_command(&mut self, cmd: DeviceRequest) {
        match cmd {
            DeviceRequest::PsCreate { request, respond_to } => {
                respond_to.send(self.handle_ps_create(request).await).ok();
            }
            DeviceRequest::Create { request, respond_to } => {
                respond_to.send(self.handle_create(*request).await).ok();
            }
            DeviceRequest::List { respond_to } => {
                respond_to.send(self.handle_list()).ok();
            }
            DeviceRequest::Update { update, respond_to } => {
                respond_to.send(self.handle_update(update).await).ok();
            }
            DeviceRequest::Delete { id, respond_to } => {
                respond_to.send(self.handle_delete(id).await).ok();
            }
            DeviceRequest::NotifyChipRemoved { chip_id, respond_to } => {
                if let Err(e) = self.handle_notify_chip_removed(chip_id) {
                    log::error!("Failed to handle NotifyChipRemoved: {}", e);
                }
                if let Some(r) = respond_to {
                    r.send(()).ok();
                }
            }
            DeviceRequest::Reset {} => {}
            DeviceRequest::GetChipStatistics { respond_to: _ } => {}
            DeviceRequest::Shutdown => {
                self.shutdown = true;
                // Shutdown of chip_clients is handled in the run loop in server.rs
            }
        }
    }

    /// Handles the `PsCreate` command.
    ///
    /// This function will either create a new device or add a chip to an
    /// existing device, based on the `device_guid` in the `params`.
    async fn handle_ps_create(&mut self, request: DevicePsCreate) -> Result<(), DeviceError> {
        let chip_id = self.new_chip_id();
        let guid = &request.device_guid;

        let device_id = self.device_ids_by_guid.get(guid).copied().unwrap_or_else(|| {
            let id = self.new_device_id();
            self.device_ids_by_guid.insert(guid.clone(), id);
            self.devices_by_id.insert(
                id,
                DeviceInfo {
                    id,
                    guid: Some(guid.clone()),
                    chips: HashSet::new(),
                    device_config: request.device_config.clone(),
                },
            );
            id
        });

        let network_kind: NetworkKind = (&request.chip_config.network_params).into();
        self.add_chip_to_device(
            device_id,
            chip_id,
            network_kind,
            request.chip_config.name.clone(),
        )?;

        self.get_chip_client(network_kind)?
            .create(ChipCreateParams {
                id: chip_id,
                packet_stream: request.packet_stream,
                packet_sink: request.packet_sink,
                config: request.chip_config,
            })
            .await?;

        if self.chip_info_map.len() == 1 {
            self.stop_idle_alarm();
        }
        Ok(())
    }

    // This is used to create user devices, like Beacon and Sniffer.
    // Those are singleton chips on a new device
    async fn handle_create(&mut self, request: api::DeviceCreate) -> Result<DeviceId, DeviceError> {
        let id = self.new_device_id();

        let device_info = DeviceInfo {
            id,
            guid: None,
            chips: HashSet::new(),
            device_config: request.device_config,
        };
        let chip_create = &request.chip;
        let chip_id = self.new_chip_id();
        let chip_params = Self::create_chip_params(chip_id, chip_create)?;
        let chip_kind = NetworkKind::from(&chip_params.config.network_params);

        self.get_chip_client(chip_kind)?.create(chip_params).await?;

        // Insert device info after successful chip creation
        self.devices_by_id.insert(id, device_info.clone());

        self.add_chip_to_device(id, chip_id, chip_kind, chip_create.name.clone())?;
        info!("Created chip {:?} for device {:?}, name {}", chip_id, id, chip_create.name);

        if self.chip_info_map.len() == 1 {
            self.stop_idle_alarm();
        }
        Ok(id)
    }

    /// Creates the `ChipCreateParams` for a new chip from the API `ChipConfig`.
    fn create_chip_params(
        id: netsim_api::chips::ChipId,
        chip_create: &api::ChipConfig,
    ) -> Result<ChipCreateParams, DeviceError> {
        let network_params = match &chip_create.chip {
            api::Chip::Beacon(beacon) => NetworkParams::Bluetooth(BluetoothParams {
                address: beacon.address.clone(),
                bt_properties: Default::default(),
                mode: BluetoothMode::Beacon(Box::new(BeaconParams { ble_beacon: beacon.clone() })),
            }),
        };

        Ok(ChipCreateParams {
            id,
            packet_stream: None,
            packet_sink: None,
            config: ChipConfig {
                name: chip_create.name.clone(),
                manufacturer: chip_create.manufacturer.clone(),
                product_name: chip_create.product_name.clone(),
                network_params,
            },
        })
    }

    /// Handles the `List` command.
    fn handle_list(&mut self) -> Result<api::ListDeviceResponse, DeviceError> {
        let devices: Vec<Device> = self
            .devices_by_id
            .values()
            .map(|device_info| Device {
                id: device_info.id.into(),
                name: device_info.device_config.name.clone(),
                visible: device_info.device_config.visible,
                position: device_info.device_config.position.clone(),
                orientation: device_info.device_config.orientation.clone(),
                // TODO: Populate chip info.
                chips: Vec::new(),
            })
            .collect();

        // TODO: use Struct not Proto for return
        Ok(api::ListDeviceResponse { devices })
    }

    /// Handles the `Update` command to modify an existing device's properties.
    async fn handle_update(&mut self, update: api::DeviceUpdate) -> Result<(), DeviceError> {
        let device_id = DeviceId(update.id);
        let mut chip_patch = ChipPatch::default();

        // 1. Acquire Lock, Perform Updates, and Extract Chip IDs inside a Scope
        let chip_ids: Vec<ChipId> = {
            let device_info = self.get_device_info(&device_id)?;

            if let Some(name) = update.name {
                device_info.device_config.name = name;
            }
            if let Some(visible) = update.visible {
                device_info.device_config.visible = visible;
            }
            if let Some(position) = update.position {
                device_info.device_config.position = position.clone();
                chip_patch.position = Some(position);
            }
            if let Some(orientation) = update.orientation {
                device_info.device_config.orientation = orientation.clone();
                chip_patch.orientation = Some(orientation);
            }
            info!("Updated device {:?} to config: {:?}", device_id, device_info.device_config);
            // Clone the Chip IDs before the lock is released
            device_info.chips.iter().cloned().collect()
        };
        // propagate changes to the chip servers (without device_info lock held)
        for chip_id in chip_ids {
            self.get_chip_client_by_id(chip_id)?.update(chip_id, chip_patch.clone()).await?;
        }
        Ok(())
    }
    /// Starts the deletion process for a user-created device.
    ///
    /// Sends delete commands to all associated chip services. The device is fully
    /// removed only after all chips confirm deletion via `NotifyChipRemoved`.
    ///
    /// Returns an error if the device ID is not found or if it's a system-managed
    /// device (e.g., created via `PsCreate`, has a GUID).
    ///
    /// TODO: Implement a timeout mechanism here. If NotifyChipRemoved isn't
    /// received within a certain period for any chip, forcefully clean up the
    /// device state to prevent hangs.
    async fn handle_delete(&mut self, id: DeviceId) -> Result<(), DeviceError> {
        let device_info = self
            .devices_by_id
            .get(&id)
            .ok_or(DeviceError::InvalidArguments(format!("Device {id} not found")))?;

        if device_info.guid.is_some() {
            return Err(DeviceError::InvalidArguments(format!(
                "Device {id} is a PsCreate device and cannot be deleted directly."
            )));
        }

        // Clone the chip IDs to avoid borrowing issues while iterating and calling async functions.
        let chip_ids: Vec<ChipId> = device_info.chips.iter().cloned().collect();

        for chip_id in chip_ids {
            self.get_chip_client_by_id(chip_id)?.delete(chip_id).await?;
        }
        info!("Initiated deletion for all chips on device {:?}", id);
        Ok(())
    }

    /// Handles the `NotifyChipRemoved` command from a chip service.
    ///
    /// This function removes the chip from the device's chip set. If the device
    /// becomes empty, it is also removed.
    fn handle_notify_chip_removed(&mut self, chip_id: ChipId) -> Result<(), DeviceError> {
        let device_id = match self.chip_info_map.remove(&chip_id) {
            Some(chip_info) => chip_info.device_id,
            None => {
                log::warn!("Chip {chip_id} not in map");
                return Ok(());
            }
        };

        if let Some(device_info) = self.devices_by_id.get_mut(&device_id) {
            if !device_info.chips.remove(&chip_id) {
                log::warn!("Chip {chip_id} device {device_id} mismatch");
            } else if device_info.chips.is_empty() {
                info!("Device {:?} became empty, removing.", device_id);
                self.delete_device(device_id);
            }
        } else {
            log::warn!("Device {device_id} for chip {chip_id} not found");
        }

        if self.chip_info_map.is_empty() {
            info!("No chips left, starting idle alarm.");
            self.start_idle_alarm();
        }
        Ok(())
    }
}
