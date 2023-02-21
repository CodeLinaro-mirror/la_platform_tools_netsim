//! Frontend-client library for rust.
///
/// Rust to C++ Grpc frontend.proto for Windows, linux and mac.
///
/// This can be replaced with grpcio native implementation when the
/// Windows build works.
#[allow(missing_docs)]
#[derive(Debug, PartialEq, Eq)]
pub enum GrpcMethod {
    GetVersion,
    UpdateDevice,
    GetDevices,
    SetPacketCapture,
    Reset,
}

#[cxx::bridge(namespace = "netsim::frontend")]
#[allow(missing_docs)]
pub mod ffi {
    // C++ types and signatures exposed to Rust.
    unsafe extern "C++" {
        include!("frontend/frontend_client.h");

        type FrontendClient;
        type ClientResult;

        #[allow(dead_code)]
        #[rust_name = "new_frontend_client"]
        pub fn NewFrontendClient() -> UniquePtr<FrontendClient>;

        #[allow(dead_code)]
        #[rust_name = "get_version"]
        pub fn GetVersion(self: &FrontendClient) -> UniquePtr<ClientResult>;

        #[allow(dead_code)]
        #[rust_name = "get_devices"]
        pub fn GetDevices(self: &FrontendClient) -> UniquePtr<ClientResult>;

        #[allow(dead_code)]
        #[rust_name = "is_ok"]
        pub fn IsOk(self: &ClientResult) -> bool;

        #[allow(dead_code)]
        #[rust_name = "err"]
        pub fn Err(self: &ClientResult) -> String;

        #[allow(dead_code)]
        #[rust_name = "byte_str"]
        pub fn ByteStr(self: &ClientResult) -> String;

    }
}
use crate::ffi::{ClientResult, FrontendClient};

/// Placeholder / temporary method before actual SendGrpc is implemented in C++
pub fn send_grpc(
    client: cxx::UniquePtr<FrontendClient>,
    grpc_method: GrpcMethod,
    request: &[u8],
) -> cxx::UniquePtr<ClientResult> {
    match grpc_method {
        GrpcMethod::GetVersion => client.get_version(),
        GrpcMethod::GetDevices => client.get_devices(),
        _ => panic!(
            "Grpc method is not implemented. grpc_method: {:#?}, request (bytes): {:?}",
            grpc_method, request
        ),
    }
}
