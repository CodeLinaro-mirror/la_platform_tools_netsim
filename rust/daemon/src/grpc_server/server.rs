// Copyright 2024 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::backend::PacketStreamerService;
use super::frontend::FrontendClient;
use grpcio::{
    ChannelBuilder, Environment, ResourceQuota, Server, ServerBuilder, ServerCredentials,
};
use log::{info, warn};
use netsim_proto::frontend_grpc::create_frontend_service;
use netsim_proto::packet_streamer_grpc::create_packet_streamer;
use std::sync::Arc;

pub fn start(port: u32) -> anyhow::Result<(Server, u16)> {
    let env = Arc::new(Environment::new(1));
    let backend_service = create_packet_streamer(PacketStreamerService);
    let frontend_service = create_frontend_service(FrontendClient);
    let quota = ResourceQuota::new(Some("NetsimGrpcServerQuota")).resize_memory(1024 * 1024);
    let ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota);
    let mut server = ServerBuilder::new(env)
        .register_service(frontend_service)
        .register_service(backend_service)
        .channel_args(ch_builder.build_args())
        .build()?;

    let addr_v4 = format!("127.0.0.1:{}", port);
    let addr_v6 = format!("[::1]:{}", port);
    let port = server.add_listening_port(addr_v4, ServerCredentials::insecure()).or_else(|e| {
        warn!("Failed to bind to 127.0.0.1:{port} in grpc server. Trying [::1]:{port}. {e:?}");
        server.add_listening_port(addr_v6, ServerCredentials::insecure())
    })?;

    server.start();
    info!("Rust gRPC listening on localhost:{port}");
    Ok((server, port))
}
