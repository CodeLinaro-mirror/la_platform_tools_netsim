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

use crate::devices::devices_handler;
use futures_util::{FutureExt as _, TryFutureExt as _};
use grpcio::{RpcContext, RpcStatus, RpcStatusCode, UnarySink};
use log::warn;
use netsim_proto::frontend::VersionResponse;
use netsim_proto::frontend_grpc::FrontendService;
use protobuf::well_known_types::empty::Empty;

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
        _ctx: grpcio::RpcContext,
        _req: netsim_proto::frontend::PatchDeviceRequest,
        _sink: grpcio::UnarySink<Empty>,
    ) {
        todo!()
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
        _ctx: grpcio::RpcContext,
        _req: netsim_proto::frontend::PatchCaptureRequest,
        _sink: grpcio::UnarySink<Empty>,
    ) {
        todo!()
    }

    fn list_capture(
        &mut self,
        _ctx: grpcio::RpcContext,
        _req: Empty,
        _sink: grpcio::UnarySink<netsim_proto::frontend::ListCaptureResponse>,
    ) {
        todo!()
    }

    fn get_capture(
        &mut self,
        _ctx: grpcio::RpcContext,
        _req: netsim_proto::frontend::GetCaptureRequest,
        _sink: grpcio::ServerStreamingSink<netsim_proto::frontend::GetCaptureResponse>,
    ) {
        todo!()
    }
}
