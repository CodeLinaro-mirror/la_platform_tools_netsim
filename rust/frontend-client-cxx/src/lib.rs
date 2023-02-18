//! Frontend-client library for rust.
///
/// Rust to C++ Grpc frontend.proto for Windows, linux and mac.
///
/// This can be replaced with grpcio native implementation when the
/// Windows build works.

#[cxx::bridge(namespace = "netsim::frontend")]
#[allow(missing_docs)]
pub mod ffi {
    // Shared enum GrpcMethod
    #[derive(Debug, PartialEq, Eq)]
    pub enum GrpcMethod {
        GetVersion,
        PatchDevice,
        GetDevices,
        Reset,
    }
    // C++ types and signatures exposed to Rust.
    unsafe extern "C++" {
        include!("frontend/frontend_client.h");

        type FrontendClient;
        type ClientResult;

        #[allow(dead_code)]
        #[rust_name = "new_frontend_client"]
        pub fn NewFrontendClient() -> UniquePtr<FrontendClient>;

        #[allow(dead_code)]
        #[rust_name = "send_grpc"]
        pub fn SendGrpc(
            self: &FrontendClient,
            grpc_method: &GrpcMethod,
            request: &Vec<u8>,
        ) -> UniquePtr<ClientResult>;

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
