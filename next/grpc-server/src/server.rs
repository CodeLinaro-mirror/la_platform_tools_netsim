// Copyright 2025 Google LLC

use std::sync::Arc;

use device_actor::DeviceClient;
use grpcio::{
    ChannelBuilder, Environment, ResourceQuota, Server, ServerBuilder, ServerCredentials,
};
use netsim_proto::{
    access_point_grpc::create_access_point_service, frontend_grpc::create_frontend_service,
    packet_streamer_grpc::create_packet_streamer,
};
use tracing::{error, info, warn};

use crate::{
    access_point::AccessPointServiceImpl, frontend::FrontendClient,
    packet_streamer::PacketStreamerService,
};

pub fn start(
    port: u32,
    device_client: DeviceClient,
    link_client: link_actor::LinkClient,
    ap_client: ap_actor::ApClient,
    packet_streamer_service: PacketStreamerService,
    version: String,
) -> Result<(Server, u16), grpcio::Error> {
    let env = Arc::new(Environment::new(1));
    let backend_service = create_packet_streamer(packet_streamer_service);
    let frontend_service =
        create_frontend_service(FrontendClient::new(device_client, Arc::new(link_client), version));
    let access_point_service = create_access_point_service(AccessPointServiceImpl::new(ap_client));
    let quota = ResourceQuota::new(Some("NetsimGrpcServerQuota")).resize_memory(1024 * 1024);
    let ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota).reuse_port(false);
    let server_builder = ServerBuilder::new(env);
    let mut server = server_builder
        .register_service(backend_service)
        .register_service(frontend_service)
        .register_service(access_point_service)
        .channel_args(ch_builder.build_args())
        .build()?;

    let addr = format!("localhost:{port}");
    let port = server.add_listening_port(&addr, ServerCredentials::insecure()).map_err(|e| {
        match std::net::TcpListener::bind(&addr) {
            Ok(listener) => drop(listener),
            Err(bind_e) => {
                if bind_e.kind() == std::io::ErrorKind::AddrInUse {
                    warn!("Rust gRPC Address {addr} is already in use.");
                } else {
                    error!("Rust gRPC bind error: {bind_e:?}")
                }
            }
        }
        e
    })?;

    server.start();
    info!("Rust gRPC listening on localhost:{port}");
    Ok((server, port))
}
