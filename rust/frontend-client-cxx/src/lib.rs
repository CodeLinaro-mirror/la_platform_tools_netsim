//! Frontend-client library for rust.
///
/// Rust to C++ Grpc frontend.proto for Windows, linux and mac.
///
/// This can be replaced with grpcio native implementation when the
/// Windows build works.

#[cxx::bridge(namespace = "netsim::frontend")]
mod ffi {
    // C++ types and signatures exposed to Rust.
    unsafe extern "C++" {
        include!("frontend/frontend_client.h");

        type ClientResult;
        type FrontendClient;

        #[allow(dead_code)]
        #[rust_name = "new_frontend_client"]
        fn NewFrontendClient() -> UniquePtr<FrontendClient>;

        #[allow(dead_code)]
        #[rust_name = "get_version"]
        fn GetVersion(self: &FrontendClient) -> UniquePtr<ClientResult>;

        #[allow(dead_code)]
        #[rust_name = "get_devices"]
        fn GetDevices(self: &FrontendClient) -> UniquePtr<ClientResult>;

    }
}
