// Copyright 2023-2025 The Android Post Source Project

use crate::server::{DeviceInfo, Server};
use log::info;
use netsim_api::{
    chips::{
        BeaconParams, BluetoothMode, BluetoothParams, ChipConfig, CreateParams as ChipCreateParams,
        NetworkKind, NetworkParams,
    },
    device_error::DeviceError,
    devices::{api, CreateDeviceParams, Device, DeviceId, DeviceRequest},
};
use std::collections::HashSet;

impl Server {
    /// This is the main entry point for handling all `DeviceRequest` commands.
    pub(crate) async fn handle_command(&mut self, cmd: DeviceRequest) {
        match cmd {
            DeviceRequest::PsCreate { params, respond_to } => {
                respond_to.send(self.handle_ps_create(params).await).ok();
            }
            DeviceRequest::Create { device, respond_to } => {
                respond_to.send(self.handle_create(*device).await).ok();
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
            DeviceRequest::Reset {} => {}
            DeviceRequest::GetChipStatistics { respond_to: _ } => {}
            DeviceRequest::Shutdown => {
                self.shutdown = true;
                // Shutdown of chip_clients is handled in the run loop in server.rs
            }
        }
    }

    // This is used to create user devices, like Beacon and Sniffer.
    // Those are singleton chips on a new device
    async fn handle_create(&mut self, request: api::DeviceCreate) -> Result<DeviceId, DeviceError> {
        let id = self.new_device_id();

        let device_info =
            DeviceInfo { id, guid: None, chips: HashSet::new(), device_config: request.config };
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

    fn create_chip_params(
        id: netsim_api::chips::ChipId,
        chip_create: &api::ChipCreate,
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

    /// Handles the `PsCreate` command.
    ///
    /// This function will either create a new device or add a chip to an
    /// existing device, based on the `device_guid` in the `params`.
    async fn handle_ps_create(&mut self, params: CreateDeviceParams) -> Result<(), DeviceError> {
        let chip_id = self.new_chip_id();
        let guid = &params.device_guid;

        let device_id = self.device_ids_by_guid.get(guid).copied().unwrap_or_else(|| {
            let id = self.new_device_id();
            self.device_ids_by_guid.insert(guid.clone(), id);
            self.devices_by_id.insert(
                id,
                DeviceInfo {
                    id,
                    guid: Some(guid.clone()),
                    chips: HashSet::new(),
                    device_config: params.device_config.clone(),
                },
            );
            id
        });

        let network_kind = (&params.chip_config.network_params).into();
        self.add_chip_to_device(device_id, chip_id, network_kind, params.chip_config.name.clone())?;

        self.get_chip_client(network_kind)?
            .create(ChipCreateParams {
                id: chip_id,
                packet_stream: params.packet_stream,
                packet_sink: params.packet_sink,
                config: params.chip_config,
            })
            .await?;

        if self.chip_info_map.len() == 1 {
            self.stop_idle_alarm();
        }
        Ok(())
    }

    async fn handle_update(&mut self, update: api::DeviceUpdate) -> Result<(), DeviceError> {
        let device_id = DeviceId(update.id);
        let device_info = self.get_device_info(&device_id)?;

        if let Some(name) = update.name {
            device_info.device_config.name = name;
        }
        if let Some(visible) = update.visible {
            device_info.device_config.visible = visible;
        }
        if let Some(position) = update.position {
            device_info.device_config.position = position;
        }
        if let Some(orientation) = update.orientation {
            device_info.device_config.orientation = orientation;
        }

        info!("Updated device {:?} to config: {:?}", device_id, device_info.device_config);
        Ok(())
    }

    async fn handle_delete(&mut self, id: DeviceId) -> Result<(), DeviceError> {
        let mut device_info = self
            .devices_by_id
            .remove(&id)
            .ok_or(DeviceError::InvalidArguments(format!("Device {id} not found")))?;
        for chip_id in device_info.chips.drain() {
            let chip_info = self
                .chip_info_map
                .remove(&chip_id)
                .ok_or(DeviceError::Internal(format!("Chip {chip_id} not found")))?;

            self.get_chip_client(chip_info.kind)?.delete(chip_id).await?;
        }
        info!("Deleted device {:?}", id);

        if self.chip_info_map.is_empty() {
            self.start_idle_alarm();
        }
        Ok(())
    }
}
