// Copyright 2023-2025 The Android Post Source Project

use crate::server::{DeviceInfo, Server};
use chrono::Utc;
use log::info;
use netsim_api::{
    chips::{
        BeaconParams, BluetoothMode, BluetoothParams, ChipConfig, CreateParams as ChipCreateParams,
        NetworkKind, NetworkParams,
    },
    device_error::DeviceError,
    devices::{api, CreateDeviceParams, DeviceId, DeviceRequest},
};
use netsim_proto::frontend::ListDeviceResponse as ProtoListDeviceResponse;
use netsim_proto::model::Device as ProtoDevice;
use protobuf::MessageField;
use std::collections::HashSet;
use std::time::SystemTime;

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
            DeviceRequest::Update { request: _ } => {}
            DeviceRequest::Delete { id: _, respond_to: _ } => {}
            DeviceRequest::Reset {} => {}
            DeviceRequest::GetChipStatistics { respond_to: _ } => {}
            DeviceRequest::Shutdown => {
                self.shutdown = true;
                self.bt_client.shutdown().await.ok();
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

        self.bt_client.create(chip_params).await?;

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
    fn handle_list(&mut self) -> Result<ProtoListDeviceResponse, DeviceError> {
        let devices: Vec<ProtoDevice> = self
            .devices_by_id
            .values()
            .map(|device_info| ProtoDevice {
                id: device_info.id.into(),
                // TODO: populate with name not guid
                name: device_info.guid.clone().unwrap_or_default(),
                visible: Some(device_info.device_config.visible),
                position: MessageField::some(device_info.device_config.position.clone()),
                orientation: MessageField::some(device_info.device_config.orientation.clone()),
                // TODO: Populate chip info.
                chips: Vec::new(),
                special_fields: Default::default(),
            })
            .collect();

        // TODO: use Struct not Proto for return
        Ok(ProtoListDeviceResponse {
            devices,
            last_modified: MessageField::some(SystemTime::from(Utc::now()).into()),
            special_fields: Default::default(),
        })
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

        self.bt_client
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
}
