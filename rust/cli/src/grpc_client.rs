//! gRPC frontend client library for netsim.
use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use grpcio::{Error as GrpcError, Result as GrpcResult, RpcStatus, RpcStatusCode};
use netsim_proto::frontend::GetCaptureRequest;
use netsim_proto::frontend_grpc::FrontendServiceClient;
use protobuf::well_known_types::empty;
use protobuf::Message;

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

// Shared enum GrpcMethod
#[derive(Debug, PartialEq, Eq)]
pub enum GrpcMethod {
    GetVersion,
    CreateDevice,
    DeleteChip,
    PatchDevice,
    ListDevice,
    Reset,
    ListCapture,
    PatchCapture,
    GetCapture,
}

pub fn get_capture(
    client: &FrontendServiceClient,
    request: &[u8],
    client_reader: &mut ClientResponseReader,
) -> Result<Vec<u8>> {
    // Use block_on to run the async block
    let mut req = GetCaptureRequest::new();
    if req.merge_from_bytes(request).is_err() {
        return Err(anyhow!(RpcStatus::new(RpcStatusCode::INVALID_ARGUMENT)));
    }
    let mut stream = client.get_capture(&req)?;

    futures::executor::block_on(async {
        // Read every available chunk from gRPC stream
        while let Some(Ok(chunk)) = stream.next().await {
            let bytes = chunk.capture_stream;
            client_reader.handle_chunk(&bytes);
        }
    });

    Ok(Vec::new())
}

pub fn send_grpc(
    client: &FrontendServiceClient,
    grpc_method: &GrpcMethod,
    request: &[u8],
) -> Result<Vec<u8>> {
    // Helper function to parse request and handle errors
    fn parse<T: Message>(request: &[u8]) -> GrpcResult<T> {
        let mut msg = T::new();
        msg.merge_from_bytes(request)
            .map_err(|_| GrpcError::RpcFailure(RpcStatus::new(RpcStatusCode::INVALID_ARGUMENT)))?;
        Ok(msg)
    }

    // Helper function to prep result
    fn prep_result<T: Message>(result: GrpcResult<T>) -> Result<Vec<u8>> {
        Ok(result?.write_to_bytes()?)
    }

    match grpc_method {
        GrpcMethod::GetVersion => prep_result(client.get_version(&empty::Empty::new())),
        GrpcMethod::CreateDevice => prep_result(client.create_device(&parse(request)?)),
        GrpcMethod::DeleteChip => prep_result(client.delete_chip(&parse(request)?)),
        GrpcMethod::PatchDevice => prep_result(client.patch_device(&parse(request)?)),
        GrpcMethod::ListDevice => prep_result(client.list_device(&empty::Empty::new())),
        GrpcMethod::Reset => prep_result(client.reset(&empty::Empty::new())),
        GrpcMethod::ListCapture => prep_result(client.list_capture(&empty::Empty::new())),
        GrpcMethod::PatchCapture => prep_result(client.patch_capture(&parse(request)?)),
        _ => Err(anyhow!(RpcStatus::new(RpcStatusCode::INVALID_ARGUMENT))),
    }
}
