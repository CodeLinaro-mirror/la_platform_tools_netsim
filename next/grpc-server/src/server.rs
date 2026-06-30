// Copyright 2026 The Android Open Source Project

// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, OnceLock};

use device_actor::DeviceClient;
use grpcio::{
    ChannelBuilder, Environment, ResourceQuota, Server, ServerBuilder, ServerCredentials,
};
#[cfg(not(feature = "cuttlefish"))]
use netsim_proto::access_point_grpc::create_access_point_service;
use netsim_proto::{
    ble_service_grpc::create_ble_service, casimir_control_grpc::create_casimir_control_service,
    cell_grpc::create_cell_service, frontend_grpc::create_frontend_service,
    nfc_service_grpc::create_nfc_service, packet_streamer_grpc::create_packet_streamer,
};
use tracing::{error, info, warn};

#[cfg(not(feature = "cuttlefish"))]
use crate::access_point::AccessPointServiceImpl;
use crate::{
    ble_service::BleServiceImpl, cell::CellServiceImpl, frontend::FrontendClient,
    nfc::NfcServiceImpl, packet_streamer::PacketStreamerService,
};

// Share a single gRPC Environment across all server instances within the same
// process (namely during parallel test execution) to prevent Abseil lock
// contention.
static SHARED_ENV: OnceLock<Arc<Environment>> = OnceLock::new();

#[allow(clippy::too_many_arguments)]
pub fn start(
    port: u32,
    #[cfg(unix)] grpc_uds_path: Option<String>,
    enable_cli_ui: bool,
    device_client: DeviceClient,
    link_client: link_actor::LinkClient,
    #[cfg(not(feature = "cuttlefish"))] ap_client: ap_actor::ApClient,
    cell_client: cell_actor::CellClient,
    nfc_client: nfc_actor::NfcClient,

    packet_streamer_service: PacketStreamerService,

    version: String,
    frontend_stats: Arc<netsim_model::FrontendStats>,
) -> Result<(Server, u16), grpcio::Error> {
    let env = SHARED_ENV.get_or_init(|| Arc::new(Environment::new(1))).clone();
    let backend_service = create_packet_streamer(packet_streamer_service);
    #[cfg(not(feature = "cuttlefish"))]
    let access_point_service =
        create_access_point_service(AccessPointServiceImpl::new(ap_client.clone()));
    let cell_service = create_cell_service(CellServiceImpl::new(cell_client));
    let casimir_service = create_casimir_control_service(
        crate::casimir::CasimirControlServiceImpl::new(nfc_client.clone()),
    );
    let nfc_service = create_nfc_service(NfcServiceImpl::new(nfc_client));

    let ble_service = create_ble_service(BleServiceImpl::new(device_client.clone()));
    let quota = ResourceQuota::new(Some("NetsimGrpcServerQuota")).resize_memory(1024 * 1024);
    let ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota).reuse_port(false);
    let mut server_builder = ServerBuilder::new(env)
        .register_service(backend_service)
        .register_service(ble_service)
        .register_service(cell_service)
        .register_service(casimir_service)
        .register_service(nfc_service);

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

    #[cfg(unix)]
    if let Some(ref uds_path) = grpc_uds_path {
        let unix_addr = format!("unix:{uds_path}");
        match server.add_listening_port(&unix_addr, ServerCredentials::insecure()) {
            Ok(0) | Err(_) => warn!("Failed to bind Rust gRPC on UDS {unix_addr}"),
            Ok(_) => info!("Rust gRPC listening on UDS {unix_addr}"),
        }
    }

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
