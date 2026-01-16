// Copyright 2025 Google LLC

use crate::frontend::FrontendClient;
use crate::packet_streamer::PacketStreamerService;
use client::{DeviceClient, LinkClient};
use grpcio::{
    ChannelBuilder, Environment, ResourceQuota, Server, ServerBuilder, ServerCredentials,
};
use log::{error, info, warn};
use netsim_proto::frontend_grpc::create_frontend_service;
use netsim_proto::packet_streamer_grpc::create_packet_streamer;
use std::sync::Arc;

pub fn start(
    port: u32,
    device_client: DeviceClient,
    link_client: LinkClient,
    packet_streamer_service: PacketStreamerService,
) -> anyhow::Result<(Server, u16)> {
    let env = Arc::new(Environment::new(1));
    let backend_service = create_packet_streamer(packet_streamer_service);
    let frontend_service = create_frontend_service(FrontendClient::new(device_client, link_client));
    let quota = ResourceQuota::new(Some("NetsimGrpcServerQuota")).resize_memory(1024 * 1024);
    let ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota).reuse_port(false);
    let server_builder = ServerBuilder::new(env);
    let mut server = server_builder
        .register_service(backend_service)
        .register_service(frontend_service)
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
