// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! gRPC frontend client library for netsim.
use futures_util::StreamExt;
use netsim_proto::{frontend, frontend_grpc::FrontendServiceClient};
use protobuf::well_known_types::empty;

use crate::error::Result;

/// Wrapper struct for application defined ClientResponseReader
pub struct ClientResponseReader {
    /// Delegated handler for reading responses
    pub handler: Box<dyn ClientResponseReadable>,
}

/// Delegating functions to handler
impl ClientResponseReader {
    fn handle_chunk(&self, chunk: &[u8]) {
        self.handler.handle_chunk(chunk);
    }
}

/// Trait for ClientResponseReader handler functions
pub trait ClientResponseReadable {
    /// Process each chunk of streaming response
    fn handle_chunk(&self, chunk: &[u8]);
}

// Enum of Grpc Requests holding the request proto as applicable
#[derive(Debug, PartialEq)]
pub enum GrpcRequest {
    GetVersion,
    ListDevice,
    Reset,
    ListCapture,
    CreateDevice(frontend::CreateDeviceRequest),
    DeleteChip(frontend::DeleteChipRequest),
    DeleteDevice(frontend::DeleteDeviceRequest),
    PatchDevice(frontend::PatchDeviceRequest),
    PatchCapture(frontend::PatchCaptureRequest),
    GetCapture(frontend::GetCaptureRequest),
    ListLink,
    CreateLink(frontend::CreateLinkRequest),
    PatchLink(frontend::PatchLinkRequest),
    DeleteLink(frontend::DeleteLinkRequest),
}

// Enum of Grpc Responses holding the response proto as applicable
#[derive(Debug, PartialEq)]
pub enum GrpcResponse {
    GetVersion(frontend::VersionResponse),
    ListDevice(frontend::ListDeviceResponse),
    Reset,
    ListCapture(frontend::ListCaptureResponse),
    CreateDevice(frontend::CreateDeviceResponse),
    DeleteChip,
    DeleteDevice,
    PatchDevice,
    PatchCapture,
    ListLink(frontend::ListLinkResponse),
    CreateLink(frontend::CreateLinkResponse),
    PatchLink,
    DeleteLink,
    Unknown,
}

pub fn get_capture(
    client: &FrontendServiceClient,
    req: &frontend::GetCaptureRequest,
    client_reader: &mut ClientResponseReader,
) -> Result<()> {
    let mut stream = client.get_capture(req)?;
    // Use block_on to run the async block handling all chunks
    futures::executor::block_on(async {
        // Read every available chunk from gRPC stream
        while let Some(Ok(chunk)) = stream.next().await {
            let bytes = chunk.capture_stream;
            client_reader.handle_chunk(&bytes);
        }
    });

    Ok(())
}

pub trait GrpcMethodExecutor {
    fn send_grpc(&self, grpc_request: &GrpcRequest) -> Result<GrpcResponse>;
}

impl GrpcMethodExecutor for FrontendServiceClient {
    fn send_grpc(&self, grpc_request: &GrpcRequest) -> Result<GrpcResponse> {
        match grpc_request {
            GrpcRequest::GetVersion => {
                Ok(GrpcResponse::GetVersion(self.get_version(&empty::Empty::new())?))
            }
            GrpcRequest::ListDevice => {
                Ok(GrpcResponse::ListDevice(self.list_device(&empty::Empty::new())?))
            }
            GrpcRequest::Reset => {
                self.reset(&empty::Empty::new())?;
                Ok(GrpcResponse::Reset)
            }
            GrpcRequest::ListCapture => {
                Ok(GrpcResponse::ListCapture(self.list_capture(&empty::Empty::new())?))
            }
            GrpcRequest::CreateDevice(req) => {
                Ok(GrpcResponse::CreateDevice(self.create_device(req)?))
            }
            GrpcRequest::DeleteChip(req) => {
                self.delete_chip(req)?;
                Ok(GrpcResponse::DeleteChip)
            }
            GrpcRequest::DeleteDevice(req) => {
                self.delete_device(req)?;
                Ok(GrpcResponse::DeleteDevice)
            }
            GrpcRequest::PatchDevice(req) => {
                self.patch_device(req)?;
                Ok(GrpcResponse::PatchDevice)
            }
            GrpcRequest::PatchCapture(req) => {
                self.patch_capture(req)?;
                Ok(GrpcResponse::PatchCapture)
            }
            GrpcRequest::ListLink => {
                Ok(GrpcResponse::ListLink(self.list_link(&empty::Empty::new())?))
            }
            GrpcRequest::CreateLink(req) => Ok(GrpcResponse::CreateLink(self.create_link(req)?)),
            GrpcRequest::PatchLink(req) => {
                self.patch_link(req)?;
                Ok(GrpcResponse::PatchLink)
            }
            GrpcRequest::DeleteLink(req) => {
                self.delete_link(req)?;
                Ok(GrpcResponse::DeleteLink)
            }
            _ => Err(grpcio::Error::RpcFailure(grpcio::RpcStatus::new(
                grpcio::RpcStatusCode::INVALID_ARGUMENT,
            ))
            .into()),
        }
    }
}

pub fn send_grpc(
    client: &FrontendServiceClient,
    grpc_request: &GrpcRequest,
) -> Result<GrpcResponse> {
    client.send_grpc(grpc_request)
}
