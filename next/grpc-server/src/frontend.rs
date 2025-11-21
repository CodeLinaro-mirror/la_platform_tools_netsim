use futures::FutureExt;
use grpcio::{RpcContext, RpcStatus, RpcStatusCode, UnarySink};
use netsim_api::devices::DeviceClient;
use netsim_proto::empty::Empty;
use netsim_proto::frontend::ListDeviceResponse;
use netsim_proto::frontend_grpc::FrontendService;
use netsim_proto::protobuf;

use crate::frontend_converter::to_proto_device;

#[derive(Clone)]
pub struct FrontendClient {
    device_client: DeviceClient,
}

impl FrontendClient {
    pub fn new(device_client: DeviceClient) -> Self {
        Self { device_client }
    }
}

impl FrontendService for FrontendClient {
    fn get_version(
        &mut self,
        ctx: RpcContext,
        _req: Empty,
        sink: UnarySink<netsim_proto::frontend::VersionResponse>,
    ) {
        let mut response = netsim_proto::frontend::VersionResponse::new();
        response.version = "0.0.1-next".to_string();
        let f = sink.success(response).map(|_| ());
        ctx.spawn(f)
    }

    fn list_device(&mut self, ctx: RpcContext, _req: Empty, sink: UnarySink<ListDeviceResponse>) {
        let client = self.device_client.clone();
        let f = async move {
            match client.list().await {
                Ok(response) => {
                    let mut proto_response = ListDeviceResponse::new();
                    for device in response.devices {
                        proto_response.devices.push(to_proto_device(device));
                    }
                    sink.success(proto_response).await
                }
                Err(e) => {
                    sink.fail(RpcStatus::with_message(
                        RpcStatusCode::INTERNAL,
                        format!("Failed to list devices: {}", e),
                    ))
                    .await
                }
            }
        }
        .map(|_| ());
        ctx.spawn(f)
    }

    fn subscribe_device(
        &mut self,
        ctx: RpcContext,
        _req: netsim_proto::frontend::SubscribeDeviceRequest,
        sink: UnarySink<netsim_proto::frontend::SubscribeDeviceResponse>,
    ) {
        let f = sink.fail(RpcStatus::new(RpcStatusCode::UNIMPLEMENTED)).map(|_| ());
        ctx.spawn(f)
    }

    fn patch_device(
        &mut self,
        ctx: RpcContext,
        req: netsim_proto::frontend::PatchDeviceRequest,
        sink: UnarySink<Empty>,
    ) {
        let client = self.device_client.clone();
        let f = async move {
            // TODO: Handle case where req.id is missing but name is provided?
            // For now, we require ID or fail if not present (or maybe 0 is invalid?)
            let id = req.id.unwrap_or(0); // 0 might be valid?

            let update = netsim_api::devices::api::DeviceUpdate {
                id,
                name: req.device.name.clone(),
                visible: req.device.visible,
                position: req
                    .device
                    .position
                    .clone()
                    .into_option()
                    .map(crate::frontend_converter::from_proto_position),
                orientation: req
                    .device
                    .orientation
                    .clone()
                    .into_option()
                    .map(crate::frontend_converter::from_proto_orientation),
            };

            match client.update(update).await {
                Ok(_) => sink.success(Empty::new()).await,
                Err(e) => {
                    sink.fail(RpcStatus::with_message(
                        RpcStatusCode::INTERNAL,
                        format!("Failed to patch device: {}", e),
                    ))
                    .await
                }
            }
        }
        .map(|_| ());
        ctx.spawn(f)
    }

    fn reset(&mut self, ctx: RpcContext, _req: Empty, sink: UnarySink<Empty>) {
        let client = self.device_client.clone();
        let f = async move {
            match client.reset().await {
                Ok(_) => sink.success(Empty::new()).await,
                Err(e) => {
                    sink.fail(RpcStatus::with_message(
                        RpcStatusCode::INTERNAL,
                        format!("Failed to reset devices: {}", e),
                    ))
                    .await
                }
            }
        }
        .map(|_| ());
        ctx.spawn(f)
    }

    fn create_device(
        &mut self,
        ctx: RpcContext,
        req: netsim_proto::frontend::CreateDeviceRequest,
        sink: UnarySink<netsim_proto::frontend::CreateDeviceResponse>,
    ) {
        let client = self.device_client.clone();
        let f = async move {
            // We only support creating a device with a single chip (Beacon) for now.
            if let Some(proto_chip) = req.device.chips.first() {
                if let Some(chip_config) =
                    crate::frontend_converter::from_proto_chip_create(proto_chip.clone())
                {
                    let device_config = netsim_api::devices::DeviceConfig {
                        name: req.device.name.clone(),
                        visible: true, // Default to true as proto doesn't have this field
                        position: crate::frontend_converter::from_proto_position(
                            req.device.position.clone().unwrap_or_default(),
                        ),
                        orientation: crate::frontend_converter::from_proto_orientation(
                            req.device.orientation.clone().unwrap_or_default(),
                        ),
                    };

                    let device_create = netsim_api::devices::api::DeviceCreate {
                        device_config: device_config.clone(),
                        chip: chip_config,
                    };

                    match client.create(Box::new(device_create)).await {
                        Ok(id) => {
                            let mut device = netsim_proto::model::Device::new();
                            device.id = id.0;
                            device.name = device_config.name;
                            device.visible = Some(device_config.visible);
                            device.position = protobuf::MessageField::some(
                                crate::frontend_converter::to_proto_position(
                                    device_config.position,
                                ),
                            );
                            device.orientation = protobuf::MessageField::some(
                                crate::frontend_converter::to_proto_orientation(
                                    device_config.orientation,
                                ),
                            );
                            // We don't have the full chip info back from create, but we can return the device skeleton.
                            // Realistically, the client might query list_device after creation.

                            sink.success(netsim_proto::frontend::CreateDeviceResponse {
                                device: protobuf::MessageField::some(device),
                                ..Default::default()
                            })
                            .await
                        }
                        Err(e) => {
                            sink.fail(RpcStatus::with_message(
                                RpcStatusCode::INTERNAL,
                                e.to_string(),
                            ))
                            .await
                        }
                    }
                } else {
                    sink.fail(RpcStatus::with_message(
                        RpcStatusCode::INVALID_ARGUMENT,
                        "Unsupported chip kind or invalid config".to_string(),
                    ))
                    .await
                }
            } else {
                sink.fail(RpcStatus::with_message(
                    RpcStatusCode::INVALID_ARGUMENT,
                    "No chips provided".to_string(),
                ))
                .await
            }
        }
        .map(|_| ());
        ctx.spawn(f)
    }

    fn delete_chip(
        &mut self,
        ctx: RpcContext,
        req: netsim_proto::frontend::DeleteChipRequest,
        sink: UnarySink<Empty>,
    ) {
        let client = self.device_client.clone();
        let f = async move {
            // Proto DeleteChipRequest has `id` which is documented as Device Identifier.
            // So this actually deletes the device?
            // netsim-api has `delete(DeviceId)`.
            match client.delete(netsim_api::devices::DeviceId(req.id)).await {
                Ok(_) => sink.success(Empty::new()).await,
                Err(e) => {
                    sink.fail(RpcStatus::with_message(
                        RpcStatusCode::INTERNAL,
                        format!("Failed to delete device: {}", e),
                    ))
                    .await
                }
            }
        }
        .map(|_| ());
        ctx.spawn(f)
    }
}
