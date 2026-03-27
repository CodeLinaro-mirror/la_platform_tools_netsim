use std::sync::Arc;

use device_actor::{DeviceClient, DeviceError};
use futures::FutureExt;
use grpcio::{RpcContext, RpcStatus, RpcStatusCode, UnarySink};
use link_api::{LinkClient, LinkCreate, LinkId, LinkUpdate};
use netsim_model::client_error::ClientError;
use netsim_proto::{
    empty::Empty,
    frontend::{ListDeviceResponse, ListLinkResponse},
    frontend_grpc::FrontendService,
    protobuf,
};

use crate::frontend_converter::to_proto_device;

#[derive(Clone)]
pub struct FrontendClient {
    device_client: DeviceClient,
    link_client: Arc<dyn LinkClient>,
    version: String,
}

impl FrontendClient {
    pub fn new(
        device_client: DeviceClient,
        link_client: Arc<dyn LinkClient>,
        version: String,
    ) -> Self {
        Self { device_client, link_client, version }
    }

    async fn handle_create_link(
        client: Arc<dyn LinkClient>,
        req: netsim_proto::frontend::CreateLinkRequest,
    ) -> Result<netsim_proto::frontend::CreateLinkResponse, RpcStatus> {
        let proto_link = req.link.into_option().ok_or_else(|| {
            RpcStatus::with_message(RpcStatusCode::INVALID_ARGUMENT, "No link provided".to_string())
        })?;

        let link = crate::frontend_converter::from_proto_link(proto_link).ok_or_else(|| {
            RpcStatus::with_message(
                RpcStatusCode::INVALID_ARGUMENT,
                "Invalid link parameters".to_string(),
            )
        })?;

        let create = LinkCreate { sender: link.sender, receiver: link.receiver, rssi: link.rssi };

        let id = client
            .create(create)
            .await
            .map_err(|e| RpcStatus::with_message(RpcStatusCode::INTERNAL, e.to_string()))?;

        let mut response = netsim_proto::frontend::CreateLinkResponse::new();
        let mut response_link = crate::frontend_converter::to_proto_link(link);
        response_link.id = id.0;
        response.link = protobuf::MessageField::some(response_link);
        Ok(response)
    }

    async fn handle_patch_link(
        client: Arc<dyn LinkClient>,
        req: netsim_proto::frontend::PatchLinkRequest,
    ) -> Result<(), RpcStatus> {
        let proto_link = req.link.into_option().ok_or_else(|| {
            RpcStatus::with_message(RpcStatusCode::INVALID_ARGUMENT, "No link provided".to_string())
        })?;

        let link = crate::frontend_converter::from_proto_link(proto_link).ok_or_else(|| {
            RpcStatus::with_message(
                RpcStatusCode::INVALID_ARGUMENT,
                "Invalid link parameters".to_string(),
            )
        })?;

        let update = LinkUpdate { rssi: Some(link.rssi) };
        client
            .update(LinkId(req.id), update)
            .await
            .map_err(|e| RpcStatus::with_message(RpcStatusCode::INTERNAL, e.to_string()))
    }

    async fn handle_delete_link(
        client: Arc<dyn LinkClient>,
        req: netsim_proto::frontend::DeleteLinkRequest,
    ) -> Result<(), RpcStatus> {
        if req.id != 0 {
            client
                .delete(LinkId(req.id))
                .await
                .map_err(|e| RpcStatus::with_message(RpcStatusCode::INTERNAL, e.to_string()))?;
            return Ok(());
        }

        Err(RpcStatus::with_message(
            RpcStatusCode::INVALID_ARGUMENT,
            "Link ID required".to_string(),
        ))
    }

    async fn handle_list_link(client: Arc<dyn LinkClient>) -> Result<ListLinkResponse, RpcStatus> {
        let links = client.list().await.map_err(|e| {
            RpcStatus::with_message(RpcStatusCode::INTERNAL, format!("Failed to list links: {}", e))
        })?;
        let mut response = ListLinkResponse::new();
        for link in links {
            response.links.push(crate::frontend_converter::to_proto_link(link));
        }
        Ok(response)
    }

    async fn handle_list_device(client: DeviceClient) -> Result<ListDeviceResponse, RpcStatus> {
        let response = client.list().await.map_err(|e| {
            RpcStatus::with_message(
                RpcStatusCode::INTERNAL,
                format!("Failed to list devices: {}", e),
            )
        })?;
        let mut proto_response = ListDeviceResponse::new();
        for device in response.devices {
            proto_response.devices.push(to_proto_device(device));
        }
        Ok(proto_response)
    }

    async fn handle_create_device(
        client: DeviceClient,
        req: netsim_proto::frontend::CreateDeviceRequest,
    ) -> Result<netsim_proto::frontend::CreateDeviceResponse, RpcStatus> {
        // We only support creating a device with a single chip (Beacon) for now.
        if let Some(proto_chip_ref) = req.device.chips.first() {
            let mut proto_chip = proto_chip_ref.clone();
            // Fallback: Use device name if chip name is missing.
            if proto_chip.name.is_empty() {
                proto_chip.name = req.device.name.clone();
            }
            if let Some(chip_config) = crate::frontend_converter::from_proto_chip_create(proto_chip)
            {
                let device_config = device_api::DeviceConfig {
                    name: req.device.name.clone(),
                    visible: true, // Default to true as proto doesn't have this field
                    position: crate::frontend_converter::from_proto_position(
                        req.device.position.clone().unwrap_or_default(),
                    ),
                    orientation: crate::frontend_converter::from_proto_orientation(
                        req.device.orientation.clone().unwrap_or_default(),
                    ),
                    builtin: false,
                    device_info: None,
                };

                let device_create = device_api::api::DeviceCreate {
                    device_config: device_config.clone(),
                    chip: chip_config,
                };

                match client.create_device(device_create).await {
                    Ok(id) => {
                        let mut device = netsim_proto::model::Device::new();
                        device.id = id.0;
                        device.name = device_config.name;
                        device.visible = Some(device_config.visible);
                        device.position = protobuf::MessageField::some(
                            crate::frontend_converter::to_proto_position(device_config.position),
                        );
                        device.orientation = protobuf::MessageField::some(
                            crate::frontend_converter::to_proto_orientation(
                                device_config.orientation,
                            ),
                        );

                        Ok(netsim_proto::frontend::CreateDeviceResponse {
                            device: protobuf::MessageField::some(device),
                            ..Default::default()
                        })
                    }
                    Err(e) => Err(RpcStatus::with_message(RpcStatusCode::INTERNAL, e.to_string())),
                }
            } else {
                Err(RpcStatus::with_message(
                    RpcStatusCode::INVALID_ARGUMENT,
                    "Unsupported chip kind or invalid config".to_string(),
                ))
            }
        } else {
            Err(RpcStatus::with_message(
                RpcStatusCode::INVALID_ARGUMENT,
                "No chips provided".to_string(),
            ))
        }
    }

    async fn handle_patch_device(
        client: DeviceClient,
        req: netsim_proto::frontend::PatchDeviceRequest,
    ) -> Result<(), RpcStatus> {
        let id = req.id.unwrap_or(0);

        let update = device_api::api::DeviceUpdate {
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
            chips: if !req.device.chips.is_empty() {
                Some(
                    req.device
                        .chips
                        .iter()
                        .cloned()
                        .map(crate::frontend_converter::from_proto_chip_update)
                        .collect(),
                )
            } else {
                None
            },
        };

        let name_opt = req.device.name.as_deref();
        client.patch(req.id, name_opt, update).await.map_err(|e| match e {
            ClientError::Framework(ref framework)
                if matches!(
                    framework.downcast_ref::<DeviceError>(),
                    Some(DeviceError::NotFound(_) | DeviceError::DeviceNotFound(_))
                ) =>
            {
                RpcStatus::with_message(
                    RpcStatusCode::NOT_FOUND,
                    format!("Device not found or patch failed: {}", e),
                )
            }
            _ => RpcStatus::with_message(
                RpcStatusCode::INTERNAL,
                format!("Failed to patch device: {}", e),
            ),
        })
    }

    #[deprecated(note = "Use handle_delete_device instead")]
    #[allow(deprecated)]
    async fn handle_delete_chip(
        _client: DeviceClient,
        _req: netsim_proto::frontend::DeleteChipRequest,
    ) -> Result<(), RpcStatus> {
        Err(RpcStatus::new(RpcStatusCode::UNIMPLEMENTED))
    }

    async fn handle_delete_device(
        client: DeviceClient,
        req: netsim_proto::frontend::DeleteDeviceRequest,
    ) -> Result<(), RpcStatus> {
        let device_id = device_api::DeviceId(req.id);
        client.delete_device(device_id).await.map_err(|e| {
            RpcStatus::with_message(
                RpcStatusCode::INTERNAL,
                format!("Failed to delete device: {}", e),
            )
        })
    }

    async fn handle_reset(client: DeviceClient) -> Result<(), RpcStatus> {
        client.reset(None).await.map_err(|e| {
            RpcStatus::with_message(
                RpcStatusCode::INTERNAL,
                format!("Failed to reset devices: {}", e),
            )
        })
    }
}

async fn reply<T>(sink: UnarySink<T>, res: Result<T, RpcStatus>) {
    match res {
        Ok(msg) => {
            let _ = sink.success(msg).await;
        }
        Err(status) => {
            let _ = sink.fail(status).await;
        }
    }
}

impl FrontendService for FrontendClient {
    fn delete_device(
        &mut self,
        ctx: RpcContext,
        _req: netsim_proto::frontend::DeleteDeviceRequest,
        sink: UnarySink<Empty>,
    ) {
        let client = self.device_client.clone();
        ctx.spawn(async move {
            let res = Self::handle_delete_device(client, _req).await.map(|_| Empty::new());
            reply(sink, res).await;
        });
    }

    fn get_version(
        &mut self,
        ctx: RpcContext,
        _req: Empty,
        sink: UnarySink<netsim_proto::frontend::VersionResponse>,
    ) {
        let mut response = netsim_proto::frontend::VersionResponse::new();
        response.version = self.version.clone();
        let f = sink.success(response).map(|_| ());
        ctx.spawn(f)
    }

    fn list_device(&mut self, ctx: RpcContext, _req: Empty, sink: UnarySink<ListDeviceResponse>) {
        let client = self.device_client.clone();
        ctx.spawn(async move {
            let res = Self::handle_list_device(client).await;
            reply(sink, res).await;
        });
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
        ctx.spawn(async move {
            let res = Self::handle_patch_device(client, req).await.map(|_| Empty::new());
            reply(sink, res).await;
        });
    }

    fn reset(&mut self, ctx: RpcContext, _req: Empty, sink: UnarySink<Empty>) {
        let client = self.device_client.clone();
        ctx.spawn(async move {
            let res = Self::handle_reset(client).await.map(|_| Empty::new());
            reply(sink, res).await;
        });
    }

    fn create_device(
        &mut self,
        ctx: RpcContext,
        req: netsim_proto::frontend::CreateDeviceRequest,
        sink: UnarySink<netsim_proto::frontend::CreateDeviceResponse>,
    ) {
        let client = self.device_client.clone();
        ctx.spawn(async move {
            let res = Self::handle_create_device(client, req).await;
            reply(sink, res).await;
        });
    }

    #[allow(deprecated)]
    fn delete_chip(
        &mut self,
        ctx: RpcContext,
        req: netsim_proto::frontend::DeleteChipRequest,
        sink: UnarySink<Empty>,
    ) {
        let client = self.device_client.clone();
        ctx.spawn(async move {
            let res = Self::handle_delete_chip(client, req).await.map(|_| Empty::new());
            reply(sink, res).await;
        });
    }

    fn create_link(
        &mut self,
        ctx: RpcContext,
        req: netsim_proto::frontend::CreateLinkRequest,
        sink: UnarySink<netsim_proto::frontend::CreateLinkResponse>,
    ) {
        let client = self.link_client.clone();
        ctx.spawn(async move {
            let res = Self::handle_create_link(client, req).await;
            reply(sink, res).await;
        });
    }

    fn list_link(&mut self, ctx: RpcContext, _req: Empty, sink: UnarySink<ListLinkResponse>) {
        let client = self.link_client.clone();
        ctx.spawn(async move {
            let res = Self::handle_list_link(client).await;
            reply(sink, res).await;
        });
    }

    fn patch_link(
        &mut self,
        ctx: RpcContext,
        req: netsim_proto::frontend::PatchLinkRequest,
        sink: UnarySink<Empty>,
    ) {
        let client = self.link_client.clone();
        ctx.spawn(async move {
            let res = Self::handle_patch_link(client, req).await.map(|_| Empty::new());
            reply(sink, res).await;
        });
    }

    fn delete_link(
        &mut self,
        ctx: RpcContext,
        req: netsim_proto::frontend::DeleteLinkRequest,
        sink: UnarySink<Empty>,
    ) {
        let client = self.link_client.clone();
        ctx.spawn(async move {
            let res = Self::handle_delete_link(client, req).await.map(|_| Empty::new());
            reply(sink, res).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use actor_framework::MockActorClient;
    use device_actor::DeviceActor;
    use link_api::{Link, LinkId, MockLinkClient};
    use netsim_model::{ChipId, ChipKind};
    use protobuf::EnumOrUnknown;

    use super::*;

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_handle_delete_chip() {
        let mock_client = MockActorClient::<DeviceActor>::new();
        let client = DeviceClient::new(Box::new(mock_client));
        let mut req = netsim_proto::frontend::DeleteChipRequest::new();
        req.id = 3;

        let res = FrontendClient::handle_delete_chip(client, req).await;
        assert_eq!(res.unwrap_err().code(), RpcStatusCode::UNIMPLEMENTED);
    }

    #[tokio::test]
    async fn test_create_link() {
        let mut client = MockLinkClient::new();
        client
            .expect_create()
            .withf(|params| params.sender.0 == 1 && params.receiver.0 == 2 && params.rssi == -50)
            .times(1)
            .returning(|_| Ok(LinkId(100)));

        let client = Arc::new(client);
        let mut req = netsim_proto::frontend::CreateLinkRequest::new();
        let mut link = netsim_proto::model::Link::new();
        link.sender_id = 1;
        link.receiver_id = 2;
        link.rssi = -50;
        link.kind = EnumOrUnknown::new(netsim_proto::common::ChipKind::BLUETOOTH);
        req.link = protobuf::MessageField::some(link);

        let resp = FrontendClient::handle_create_link(client.clone(), req).await.unwrap();
        assert_eq!(resp.link.id, 100);
    }

    #[tokio::test]
    async fn test_patch_link() {
        let mut client = MockLinkClient::new();
        // Update()
        client
            .expect_update()
            .withf(|id, patch| id.0 == 10 && patch.rssi == Some(-60))
            .times(1)
            .returning(|_, _| Ok(()));

        let client = Arc::new(client);
        let mut req = netsim_proto::frontend::PatchLinkRequest::new();
        req.id = 10;
        let mut link = netsim_proto::model::Link::new();
        link.sender_id = 1;
        link.receiver_id = 2;
        link.rssi = -60;
        link.kind = EnumOrUnknown::new(netsim_proto::common::ChipKind::BLUETOOTH);
        req.link = protobuf::MessageField::some(link);

        FrontendClient::handle_patch_link(client.clone(), req).await.unwrap();
    }

    #[tokio::test]
    async fn test_delete_link() {
        let mut client = MockLinkClient::new();
        // delete()
        client.expect_delete().withf(|id| id.0 == 20).times(1).returning(|_| Ok(()));

        let client = Arc::new(client);
        let mut req = netsim_proto::frontend::DeleteLinkRequest::new();
        req.id = 20;

        FrontendClient::handle_delete_link(client.clone(), req).await.unwrap();
    }

    #[tokio::test]
    async fn test_list_link() {
        let mut client = MockLinkClient::new();
        client.expect_list().times(1).returning(|| {
            Ok(vec![Link {
                id: LinkId(1),
                sender: ChipId(10),
                receiver: ChipId(11),
                kind: ChipKind::BLUETOOTH,
                rssi: -70,
            }])
        });

        let client = Arc::new(client);
        let resp = FrontendClient::handle_list_link(client.clone()).await.unwrap();
        assert_eq!(resp.links.len(), 1);
        assert_eq!(resp.links[0].id, 1);
        assert_eq!(resp.links[0].sender_id, 10);
        assert_eq!(resp.links[0].receiver_id, 11);
    }
}
