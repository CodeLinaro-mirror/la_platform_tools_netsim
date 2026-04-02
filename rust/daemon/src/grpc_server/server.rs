// Copyright 2024 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use super::backend::PacketStreamerService;
use super::frontend::FrontendClient;
use crate::links::link::LinkManager;
use grpcio::{
    ChannelBuilder, Environment, ResourceQuota, Server, ServerBuilder, ServerCredentials,
};
use log::{error, info, warn};
use netsim_proto::frontend_grpc::create_frontend_service;
use netsim_proto::packet_streamer_grpc::create_packet_streamer;
use std::sync::Arc;

pub fn start(
    port: u32,
    no_cli_ui: bool,
    link_manager: Arc<LinkManager>,
    _vsock: u16,
) -> anyhow::Result<(Server, u16)> {
    let env = Arc::new(Environment::new(1));
    let backend_service = create_packet_streamer(PacketStreamerService);
    let frontend_service = create_frontend_service(FrontendClient::new(link_manager));
    let quota = ResourceQuota::new(Some("NetsimGrpcServerQuota")).resize_memory(1024 * 1024);
    let ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota).reuse_port(false);
    let mut server_builder = ServerBuilder::new(env);
    if !no_cli_ui {
        server_builder = server_builder.register_service(frontend_service);
    }
    let mut server = server_builder
        .register_service(backend_service)
        .channel_args(ch_builder.build_args())
        .build()?;

    let addr = format!("localhost:{port}");
    #[allow(clippy::manual_inspect)]
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

    #[cfg(feature = "cuttlefish")]
    if _vsock != 0 {
        let vsock_uri = format!("vsock:{}:{}", libc::VMADDR_CID_ANY, _vsock);
        info!("vsock_uri: {vsock_uri}");
        server.add_listening_port(vsock_uri, ServerCredentials::insecure())?;
    }

    server.start();
    info!("Rust gRPC listening on localhost:{port}");
    Ok((server, port))
}
