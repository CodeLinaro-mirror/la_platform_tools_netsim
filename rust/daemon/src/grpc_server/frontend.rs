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

use crate::captures::captures_handler;
use crate::devices::chip::ChipIdentifier;
use crate::devices::devices_handler;
use futures_util::{FutureExt as _, SinkExt as _, TryFutureExt as _};
use grpcio::{RpcContext, RpcStatus, RpcStatusCode, UnarySink, WriteFlags};
use log::warn;
use netsim_proto::frontend::VersionResponse;
use netsim_proto::frontend_grpc::FrontendService;
use protobuf::well_known_types::empty::Empty;

use std::io::Read;

#[derive(Clone)]
pub struct FrontendClient;

impl FrontendService for FrontendClient {
    fn get_version(&mut self, ctx: RpcContext<'_>, req: Empty, sink: UnarySink<VersionResponse>) {
        let response =
            VersionResponse { version: crate::version::get_version(), ..Default::default() };
        let f = sink
            .success(response)
            .map_err(move |e| eprintln!("client error {:?}: {:?}", req, e))
            .map(|_| ());
        ctx.spawn(f)
    }

    fn list_device(
        &mut self,
        ctx: grpcio::RpcContext,
        req: Empty,
        sink: grpcio::UnarySink<netsim_proto::frontend::ListDeviceResponse>,
    ) {
        let response = match devices_handler::list_device() {
            Ok(response) => sink.success(response),
            Err(e) => {
                warn!("failed to list device: {}", e);
                sink.fail(RpcStatus::with_message(RpcStatusCode::INTERNAL, e))
            }
        };

        ctx.spawn(response.map_err(move |e| warn!("client error {:?}: {:?}", req, e)).map(|_| ()))
    }

    fn patch_device(
        &mut self,
        ctx: grpcio::RpcContext,
        req: netsim_proto::frontend::PatchDeviceRequest,
        sink: grpcio::UnarySink<Empty>,
    ) {
        let response = match devices_handler::patch_device(req) {
            Ok(_) => sink.success(Empty::new()),
            Err(e) => {
                warn!("failed to patch device: {}", e);
                sink.fail(RpcStatus::with_message(RpcStatusCode::INTERNAL, e))
            }
        };

        ctx.spawn(response.map_err(move |e| warn!("client error: {:?}", e)).map(|_| ()))
    }

    fn reset(&mut self, ctx: grpcio::RpcContext, _req: Empty, sink: grpcio::UnarySink<Empty>) {
        let response = match devices_handler::reset_all() {
            Ok(_) => sink.success(Empty::new()),
            Err(e) => {
                warn!("failed to reset: {}", e);
                sink.fail(RpcStatus::with_message(RpcStatusCode::INTERNAL, e))
            }
        };

        ctx.spawn(response.map_err(move |e| warn!("client error: {:?}", e)).map(|_| ()))
    }

    fn patch_capture(
        &mut self,
        ctx: grpcio::RpcContext,
        req: netsim_proto::frontend::PatchCaptureRequest,
        sink: grpcio::UnarySink<Empty>,
    ) {
        let id = req.id;
        let state = match req.patch.state {
            Some(v) => v,
            None => {
                let error_msg = "Capture patch state not provided";
                warn!("{}", error_msg);
                sink.fail(RpcStatus::with_message(
                    RpcStatusCode::INVALID_ARGUMENT,
                    error_msg.to_string(),
                ));
                return;
            }
        };

        let response = match captures_handler::patch_capture(ChipIdentifier(id), state) {
            Ok(_) => sink.success(Empty::new()),
            Err(e) => {
                warn!("failed to patch capture: {}", e);
                sink.fail(RpcStatus::with_message(RpcStatusCode::INTERNAL, e.to_string()))
            }
        };

        ctx.spawn(response.map_err(move |e| warn!("client error: {:?}", e)).map(|_| ()))
    }

    fn list_capture(
        &mut self,
        ctx: grpcio::RpcContext,
        req: Empty,
        sink: grpcio::UnarySink<netsim_proto::frontend::ListCaptureResponse>,
    ) {
        let response = match captures_handler::list_capture() {
            Ok(response) => sink.success(response),
            Err(e) => {
                warn!("failed to list capture: {}", e);
                sink.fail(RpcStatus::with_message(RpcStatusCode::INTERNAL, e.to_string()))
            }
        };

        ctx.spawn(response.map_err(move |e| warn!("client error {:?}: {:?}", req, e)).map(|_| ()))
    }

    fn get_capture(
        &mut self,
        ctx: grpcio::RpcContext,
        req: netsim_proto::frontend::GetCaptureRequest,
        mut sink: grpcio::ServerStreamingSink<netsim_proto::frontend::GetCaptureResponse>,
    ) {
        let mut file = match captures_handler::get_capture(ChipIdentifier(req.id)) {
            Ok(f) => f,
            Err(e) => {
                warn!("failed to get capture: {}", e);
                return ctx.spawn(
                    sink.fail(RpcStatus::with_message(RpcStatusCode::INTERNAL, e.to_string()))
                        .map_err(move |e| warn!("client error {:?}: {:?}", req, e))
                        .map(|_| ()),
                );
            }
        };

        let mut buffer = [0u8; captures_handler::CHUNK_LEN];

        let f = async move {
            loop {
                let length = match file.read(&mut buffer) {
                    Ok(l) => l,
                    Err(e) => {
                        warn!("failed to read file: {}", e);
                        sink.fail(RpcStatus::with_message(RpcStatusCode::INTERNAL, e.to_string()))
                            .await?;
                        return Ok(());
                    }
                };
                if length == 0 {
                    break;
                }
                let mut response = netsim_proto::frontend::GetCaptureResponse::new();
                response.capture_stream = buffer[..length].to_vec(); // Send only read data
                sink.send((response, WriteFlags::default())).await?;
            }
            sink.close().await?;
            Ok(())
        }
        .map_err(|e: grpcio::Error| log::error!("failed to handle get_capture request: {:?}", e))
        .map(|_| ());
        ctx.spawn(f)
    }

    fn create_device(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: netsim_proto::frontend::CreateDeviceRequest,
        sink: ::grpcio::UnarySink<netsim_proto::frontend::CreateDeviceResponse>,
    ) {
        let response = match devices_handler::create_device(&req) {
            Ok(device_proto) => sink.success(netsim_proto::frontend::CreateDeviceResponse {
                device: protobuf::MessageField::some(device_proto),
                ..Default::default()
            }),
            Err(e) => {
                warn!("failed to create chip: {}", e);
                sink.fail(RpcStatus::with_message(RpcStatusCode::INTERNAL, e.to_string()))
            }
        };
        ctx.spawn(response.map_err(move |e| warn!("client error: {:?}", e)).map(|_| ()))
    }

    fn delete_chip(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: netsim_proto::frontend::DeleteChipRequest,
        sink: ::grpcio::UnarySink<Empty>,
    ) {
        let response = match devices_handler::delete_chip(&req) {
            Ok(()) => sink.success(Empty::new()),
            Err(e) => {
                warn!("failed to delete chip: {}", e);
                sink.fail(RpcStatus::with_message(RpcStatusCode::INTERNAL, e.to_string()))
            }
        };

        ctx.spawn(response.map_err(move |e| warn!("client error: {:?}", e)).map(|_| ()))
    }
}
