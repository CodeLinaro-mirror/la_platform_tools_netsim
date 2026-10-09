// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unused_imports)]
#![allow(unused_results)]

const METHOD_SERVER_REFLECTION_SERVER_REFLECTION_INFO: ::grpcio::Method<
    super::reflection::ServerReflectionRequest,
    super::reflection::ServerReflectionResponse,
> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Duplex,
    name: "/grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct ServerReflectionClient {
    pub client: ::grpcio::Client,
}

impl ServerReflectionClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        ServerReflectionClient { client: ::grpcio::Client::new(channel) }
    }

    pub fn server_reflection_info_opt(
        &self,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<(
        ::grpcio::ClientDuplexSender<super::reflection::ServerReflectionRequest>,
        ::grpcio::ClientDuplexReceiver<super::reflection::ServerReflectionResponse>,
    )> {
        self.client.duplex_streaming(&METHOD_SERVER_REFLECTION_SERVER_REFLECTION_INFO, opt)
    }

    pub fn server_reflection_info(
        &self,
    ) -> ::grpcio::Result<(
        ::grpcio::ClientDuplexSender<super::reflection::ServerReflectionRequest>,
        ::grpcio::ClientDuplexReceiver<super::reflection::ServerReflectionResponse>,
    )> {
        self.server_reflection_info_opt(::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F)
    where
        F: ::std::future::Future<Output = ()> + Send + 'static,
    {
        self.client.spawn(f)
    }
}

pub trait ServerReflection {
    fn server_reflection_info(
        &mut self,
        ctx: ::grpcio::RpcContext,
        _stream: ::grpcio::RequestStream<super::reflection::ServerReflectionRequest>,
        sink: ::grpcio::DuplexSink<super::reflection::ServerReflectionResponse>,
    ) {
        grpcio::unimplemented_call!(ctx, sink)
    }
}

pub fn create_server_reflection<S: ServerReflection + Send + Clone + 'static>(
    s: S,
) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s;
    builder = builder.add_duplex_streaming_handler(
        &METHOD_SERVER_REFLECTION_SERVER_REFLECTION_INFO,
        move |ctx, req, resp| instance.server_reflection_info(ctx, req, resp),
    );
    builder.build()
}
