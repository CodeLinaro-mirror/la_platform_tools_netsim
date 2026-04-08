// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, OnceLock};

use device_actor::DeviceClient;
use grpcio::{
    ChannelBuilder, Environment, ResourceQuota, Server, ServerBuilder, ServerCredentials,
};
use netsim_proto::{
    access_point_grpc::create_access_point_service, ble_service_grpc::create_ble_service,
    frontend_grpc::create_frontend_service, packet_streamer_grpc::create_packet_streamer,
};
use tracing::{error, info, warn};

use crate::{
    access_point::AccessPointServiceImpl, ble_service::BleServiceImpl, frontend::FrontendClient,
    packet_streamer::PacketStreamerService,
};

// Share a single gRPC Environment across all server instances within the same
// process (namely during parallel test execution) to prevent Abseil lock
// contention.
static SHARED_ENV: OnceLock<Arc<Environment>> = OnceLock::new();

pub fn start(
    port: u32,
    device_client: DeviceClient,
    link_client: link_actor::LinkClient,
    ap_client: ap_actor::ApClient,
    packet_streamer_service: PacketStreamerService,
    version: String,
) -> Result<(Server, u16), grpcio::Error> {
    let env = SHARED_ENV.get_or_init(|| Arc::new(Environment::new(1))).clone();
    let backend_service = create_packet_streamer(packet_streamer_service);
    let frontend_service = create_frontend_service(FrontendClient::new(
        device_client.clone(),
        Arc::new(link_client),
        version,
    ));
    let access_point_service = create_access_point_service(AccessPointServiceImpl::new(ap_client));
    let ble_service = create_ble_service(BleServiceImpl::new(device_client.clone()));
    let quota = ResourceQuota::new(Some("NetsimGrpcServerQuota")).resize_memory(1024 * 1024);
    let ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota).reuse_port(false);
    let server_builder = ServerBuilder::new(env);
    let mut server = server_builder
        .register_service(backend_service)
        .register_service(frontend_service)
        .register_service(access_point_service)
        .register_service(ble_service)
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
