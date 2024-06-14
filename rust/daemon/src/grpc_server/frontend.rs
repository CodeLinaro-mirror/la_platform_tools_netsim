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

use futures_util::{FutureExt as _, TryFutureExt as _};
use grpcio::{RpcContext, UnarySink};
use netsim_proto::frontend::VersionResponse;
use netsim_proto::frontend_grpc::FrontendService;

#[derive(Clone)]
pub struct FrontendClient;

impl FrontendService for FrontendClient {
    fn get_version(
        &mut self,
        ctx: RpcContext<'_>,
        req: protobuf::well_known_types::empty::Empty,
        sink: UnarySink<VersionResponse>,
    ) {
        let response = VersionResponse {
            version: "netsim test server version 0.0.1".to_string(),
            ..Default::default()
        };
        let f = sink
            .success(response)
            .map_err(move |e| eprintln!("failed to reply {:?}: {:?}", req, e))
            .map(|_| ());
        ctx.spawn(f)
    }

    fn list_device(
        &mut self,
        _ctx: grpcio::RpcContext,
        _req: protobuf::well_known_types::empty::Empty,
        _sink: grpcio::UnarySink<netsim_proto::frontend::ListDeviceResponse>,
    ) {
        todo!()
    }

    fn patch_device(
        &mut self,
        _ctx: grpcio::RpcContext,
        _req: netsim_proto::frontend::PatchDeviceRequest,
        _sink: grpcio::UnarySink<protobuf::well_known_types::empty::Empty>,
    ) {
        todo!()
    }

    fn reset(
        &mut self,
        _ctx: grpcio::RpcContext,
        _req: protobuf::well_known_types::empty::Empty,
        _sink: grpcio::UnarySink<protobuf::well_known_types::empty::Empty>,
    ) {
        todo!()
    }

    fn patch_capture(
        &mut self,
        _ctx: grpcio::RpcContext,
        _req: netsim_proto::frontend::PatchCaptureRequest,
        _sink: grpcio::UnarySink<protobuf::well_known_types::empty::Empty>,
    ) {
        todo!()
    }

    fn list_capture(
        &mut self,
        _ctx: grpcio::RpcContext,
        _req: protobuf::well_known_types::empty::Empty,
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
