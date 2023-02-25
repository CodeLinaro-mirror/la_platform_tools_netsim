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
    PatchDevice,
    GetDevices,
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
        #[rust_name = "reset"]
        pub fn Reset(self: &FrontendClient) -> UniquePtr<ClientResult>;

        #[allow(dead_code)]
        #[rust_name = "patch_device"]
        pub fn PatchDevice(self: &FrontendClient, request: &Vec<u8>) -> UniquePtr<ClientResult>;

        #[allow(dead_code)]
        #[rust_name = "is_ok"]
        pub fn IsOk(self: &ClientResult) -> bool;

        #[allow(dead_code)]
        #[rust_name = "err"]
        pub fn Err(self: &ClientResult) -> String;

        #[allow(dead_code)]
        #[rust_name = "byte_vec"]
        pub fn ByteVec(self: &ClientResult) -> &CxxVector<u8>;

    }
}
use crate::ffi::{ClientResult, FrontendClient};

/// Placeholder / temporary method before actual SendGrpc is implemented in C++
pub fn send_grpc(
    client: &cxx::UniquePtr<FrontendClient>,
    grpc_method: &GrpcMethod,
    request: &Vec<u8>,
) -> cxx::UniquePtr<ClientResult> {
    match grpc_method {
        GrpcMethod::GetVersion => client.get_version(),
        GrpcMethod::GetDevices => client.get_devices(),
        GrpcMethod::Reset => client.reset(),
        GrpcMethod::PatchDevice => client.patch_device(request),
    }
}
