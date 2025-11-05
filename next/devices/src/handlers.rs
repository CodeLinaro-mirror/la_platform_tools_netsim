// Copyright 2023-2025 The Android Post Source Project

use crate::server::{DeviceInfo, Server};
use chrono::Utc;
use netsim_api::{
    chips::CreateParams,
    device_error::DeviceError,
    devices::{CreateDeviceParams, DeviceRequest},
};
use netsim_proto::frontend::ListDeviceResponse;
use netsim_proto::model;
use protobuf::MessageField;
use std::collections::HashSet;
use std::time::SystemTime;

impl Server {
    /// This is the main entry point for handling all `DeviceRequest` commands.
    pub(super) async fn handle_command(&mut self, cmd: DeviceRequest) {
        match cmd {
            DeviceRequest::PsCreate { params, respond_to } => {
                respond_to.send(self.handle_ps_create(params).await).ok();
            }
            DeviceRequest::Create { device: _, respond_to: _ } => {}
            DeviceRequest::List { respond_to } => {
                respond_to.send(Ok(self.handle_list())).ok();
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

    /// Handles the `List` command.
    fn handle_list(&mut self) -> ListDeviceResponse {
        let devices: Vec<model::Device> = self
            .devices_by_id
            .values()
            .map(|device_info| model::Device {
                id: device_info.id.into(),
                // TODO: populate with name not guid
                name: device_info.guid.clone(),
                visible: Some(device_info.device_config.visible),
                position: MessageField::some(device_info.device_config.position.clone()),
                orientation: MessageField::some(device_info.device_config.orientation.clone()),
                // TODO: Populate chip info.
                chips: Vec::new(),
                special_fields: Default::default(),
            })
            .collect();

        // TODO: use Struct not Proto for return
        ListDeviceResponse {
            devices,
            last_modified: MessageField::some(SystemTime::from(Utc::now()).into()),
            special_fields: Default::default(),
        }
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
                    guid: guid.clone(),
                    chips: HashSet::new(),
                    device_config: params.device_config.clone(),
                },
            );
            id
        });

        self.get_device_info(&device_id)?.chips.insert(chip_id);

        let network_kind = (&params.chip_config.network_params).into();
        self.chip_to_device_map.insert(chip_id, (network_kind, device_id));

        self.bt_client
            .create(CreateParams {
                id: chip_id,
                packet_stream: params.packet_stream,
                packet_sink: params.packet_sink,
                config: params.chip_config,
            })
            .await?;

        if self.chip_to_device_map.len() == 1 {
            self.stop_idle_alarm();
        }
        Ok(())
    }
}
