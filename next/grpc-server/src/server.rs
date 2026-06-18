// Copyright 2026 The Android Open Source Project

// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, OnceLock};

use device_actor::DeviceClient;
use grpcio::{
    ChannelBuilder, Environment, ResourceQuota, Server, ServerBuilder, ServerCredentials,
};
#[cfg(not(feature = "cuttlefish"))]
use netsim_proto::access_point_grpc::create_access_point_service;
#[cfg(not(feature = "cuttlefish"))]
use netsim_proto::cell_grpc::create_cell_service;
use netsim_proto::{
    ble_service_grpc::create_ble_service, frontend_grpc::create_frontend_service,
    packet_streamer_grpc::create_packet_streamer,
};
use tracing::{error, info, warn};

#[cfg(not(feature = "cuttlefish"))]
use crate::access_point::AccessPointServiceImpl;
#[cfg(not(feature = "cuttlefish"))]
use crate::cell::CellServiceImpl;
use crate::{
    ble_service::BleServiceImpl, frontend::FrontendClient, packet_streamer::PacketStreamerService,
};

// Share a single gRPC Environment across all server instances within the same
// process (namely during parallel test execution) to prevent Abseil lock
// contention.
static SHARED_ENV: OnceLock<Arc<Environment>> = OnceLock::new();

#[allow(clippy::too_many_arguments)]
pub fn start(
    port: u32,
    enable_cli_ui: bool,
    device_client: DeviceClient,
    link_client: link_actor::LinkClient,
    #[cfg(not(feature = "cuttlefish"))] ap_client: ap_actor::ApClient,

    packet_streamer_service: PacketStreamerService,

    version: String,
    frontend_stats: Arc<netsim_model::FrontendStats>,
) -> Result<(Server, u16), grpcio::Error> {
    let env = SHARED_ENV.get_or_init(|| Arc::new(Environment::new(1))).clone();
    let backend_service = create_packet_streamer(packet_streamer_service);
    #[cfg(not(feature = "cuttlefish"))]
    let access_point_service =
        create_access_point_service(AccessPointServiceImpl::new(ap_client.clone()));
    #[cfg(not(feature = "cuttlefish"))]
    let cell_service = create_cell_service(CellServiceImpl::new());

    let ble_service = create_ble_service(BleServiceImpl::new(device_client.clone()));
    let quota = ResourceQuota::new(Some("NetsimGrpcServerQuota")).resize_memory(1024 * 1024);
    let ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota).reuse_port(false);
    let mut server_builder =
        ServerBuilder::new(env).register_service(backend_service).register_service(ble_service);

    #[cfg(not(feature = "cuttlefish"))]
    {
        server_builder = server_builder.register_service(cell_service);
    }

    #[cfg(not(feature = "cuttlefish"))]
    {
        server_builder = server_builder.register_service(access_point_service);
    }

    if enable_cli_ui {
        let frontend_service = create_frontend_service(FrontendClient::new(
            device_client.clone(),
            Arc::new(link_client),
            #[cfg(not(feature = "cuttlefish"))]
            ap_client,
            version,
            frontend_stats,
        ));
        server_builder = server_builder.register_service(frontend_service);
    }

    let mut server = server_builder.channel_args(ch_builder.build_args()).build()?;

    let addr = format!("localhost:{port}");
    let port =
        server.add_listening_port(&addr, ServerCredentials::insecure()).inspect_err(|_| {
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
        })?;

    server.start();
    info!("Rust gRPC listening on localhost:{port}");
    Ok((server, port))
}
