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
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]

const METHOD_CASIMIR_CONTROL_SERVICE_SEND_APDU: ::grpcio::Method<
    super::casimircontrolserver::SendApduRequest,
    super::casimircontrolserver::SendApduReply,
> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/casimircontrolserver.CasimirControlService/SendApdu",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_CASIMIR_CONTROL_SERVICE_POLL_A: ::grpcio::Method<
    super::casimircontrolserver::Void,
    super::casimircontrolserver::SenderId,
> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/casimircontrolserver.CasimirControlService/PollA",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_CASIMIR_CONTROL_SERVICE_SET_RADIO_STATE: ::grpcio::Method<
    super::casimircontrolserver::RadioState,
    super::casimircontrolserver::Void,
> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/casimircontrolserver.CasimirControlService/SetRadioState",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_CASIMIR_CONTROL_SERVICE_SET_POWER_LEVEL: ::grpcio::Method<
    super::casimircontrolserver::PowerLevel,
    super::casimircontrolserver::Void,
> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/casimircontrolserver.CasimirControlService/SetPowerLevel",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_CASIMIR_CONTROL_SERVICE_SEND_BROADCAST: ::grpcio::Method<
    super::casimircontrolserver::SendBroadcastRequest,
    super::casimircontrolserver::SendBroadcastResponse,
> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/casimircontrolserver.CasimirControlService/SendBroadcast",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_CASIMIR_CONTROL_SERVICE_INIT: ::grpcio::Method<
    super::casimircontrolserver::Void,
    super::casimircontrolserver::Void,
> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/casimircontrolserver.CasimirControlService/Init",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_CASIMIR_CONTROL_SERVICE_CLOSE: ::grpcio::Method<
    super::casimircontrolserver::Void,
    super::casimircontrolserver::Void,
> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/casimircontrolserver.CasimirControlService/Close",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct CasimirControlServiceClient {
    pub client: ::grpcio::Client,
}

impl CasimirControlServiceClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        CasimirControlServiceClient { client: ::grpcio::Client::new(channel) }
    }

    pub fn send_apdu_opt(
        &self,
        req: &super::casimircontrolserver::SendApduRequest,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<super::casimircontrolserver::SendApduReply> {
        self.client.unary_call(&METHOD_CASIMIR_CONTROL_SERVICE_SEND_APDU, req, opt)
    }

    pub fn send_apdu(
        &self,
        req: &super::casimircontrolserver::SendApduRequest,
    ) -> ::grpcio::Result<super::casimircontrolserver::SendApduReply> {
        self.send_apdu_opt(req, ::grpcio::CallOption::default())
    }

    pub fn send_apdu_async_opt(
        &self,
        req: &super::casimircontrolserver::SendApduRequest,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::casimircontrolserver::SendApduReply>>
    {
        self.client.unary_call_async(&METHOD_CASIMIR_CONTROL_SERVICE_SEND_APDU, req, opt)
    }

    pub fn send_apdu_async(
        &self,
        req: &super::casimircontrolserver::SendApduRequest,
    ) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::casimircontrolserver::SendApduReply>>
    {
        self.send_apdu_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn poll_a_opt(
        &self,
        req: &super::casimircontrolserver::Void,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<super::casimircontrolserver::SenderId> {
        self.client.unary_call(&METHOD_CASIMIR_CONTROL_SERVICE_POLL_A, req, opt)
    }

    pub fn poll_a(
        &self,
        req: &super::casimircontrolserver::Void,
    ) -> ::grpcio::Result<super::casimircontrolserver::SenderId> {
        self.poll_a_opt(req, ::grpcio::CallOption::default())
    }

    pub fn poll_a_async_opt(
        &self,
        req: &super::casimircontrolserver::Void,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::casimircontrolserver::SenderId>>
    {
        self.client.unary_call_async(&METHOD_CASIMIR_CONTROL_SERVICE_POLL_A, req, opt)
    }

    pub fn poll_a_async(
        &self,
        req: &super::casimircontrolserver::Void,
    ) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::casimircontrolserver::SenderId>>
    {
        self.poll_a_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_radio_state_opt(
        &self,
        req: &super::casimircontrolserver::RadioState,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<super::casimircontrolserver::Void> {
        self.client.unary_call(&METHOD_CASIMIR_CONTROL_SERVICE_SET_RADIO_STATE, req, opt)
    }

    pub fn set_radio_state(
        &self,
        req: &super::casimircontrolserver::RadioState,
    ) -> ::grpcio::Result<super::casimircontrolserver::Void> {
        self.set_radio_state_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_radio_state_async_opt(
        &self,
        req: &super::casimircontrolserver::RadioState,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::casimircontrolserver::Void>> {
        self.client.unary_call_async(&METHOD_CASIMIR_CONTROL_SERVICE_SET_RADIO_STATE, req, opt)
    }

    pub fn set_radio_state_async(
        &self,
        req: &super::casimircontrolserver::RadioState,
    ) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::casimircontrolserver::Void>> {
        self.set_radio_state_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_power_level_opt(
        &self,
        req: &super::casimircontrolserver::PowerLevel,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<super::casimircontrolserver::Void> {
        self.client.unary_call(&METHOD_CASIMIR_CONTROL_SERVICE_SET_POWER_LEVEL, req, opt)
    }

    pub fn set_power_level(
        &self,
        req: &super::casimircontrolserver::PowerLevel,
    ) -> ::grpcio::Result<super::casimircontrolserver::Void> {
        self.set_power_level_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_power_level_async_opt(
        &self,
        req: &super::casimircontrolserver::PowerLevel,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::casimircontrolserver::Void>> {
        self.client.unary_call_async(&METHOD_CASIMIR_CONTROL_SERVICE_SET_POWER_LEVEL, req, opt)
    }

    pub fn set_power_level_async(
        &self,
        req: &super::casimircontrolserver::PowerLevel,
    ) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::casimircontrolserver::Void>> {
        self.set_power_level_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn send_broadcast_opt(
        &self,
        req: &super::casimircontrolserver::SendBroadcastRequest,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<super::casimircontrolserver::SendBroadcastResponse> {
        self.client.unary_call(&METHOD_CASIMIR_CONTROL_SERVICE_SEND_BROADCAST, req, opt)
    }

    pub fn send_broadcast(
        &self,
        req: &super::casimircontrolserver::SendBroadcastRequest,
    ) -> ::grpcio::Result<super::casimircontrolserver::SendBroadcastResponse> {
        self.send_broadcast_opt(req, ::grpcio::CallOption::default())
    }

    pub fn send_broadcast_async_opt(
        &self,
        req: &super::casimircontrolserver::SendBroadcastRequest,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<super::casimircontrolserver::SendBroadcastResponse>,
    > {
        self.client.unary_call_async(&METHOD_CASIMIR_CONTROL_SERVICE_SEND_BROADCAST, req, opt)
    }

    pub fn send_broadcast_async(
        &self,
        req: &super::casimircontrolserver::SendBroadcastRequest,
    ) -> ::grpcio::Result<
        ::grpcio::ClientUnaryReceiver<super::casimircontrolserver::SendBroadcastResponse>,
    > {
        self.send_broadcast_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn init_opt(
        &self,
        req: &super::casimircontrolserver::Void,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<super::casimircontrolserver::Void> {
        self.client.unary_call(&METHOD_CASIMIR_CONTROL_SERVICE_INIT, req, opt)
    }

    pub fn init(
        &self,
        req: &super::casimircontrolserver::Void,
    ) -> ::grpcio::Result<super::casimircontrolserver::Void> {
        self.init_opt(req, ::grpcio::CallOption::default())
    }

    pub fn init_async_opt(
        &self,
        req: &super::casimircontrolserver::Void,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::casimircontrolserver::Void>> {
        self.client.unary_call_async(&METHOD_CASIMIR_CONTROL_SERVICE_INIT, req, opt)
    }

    pub fn init_async(
        &self,
        req: &super::casimircontrolserver::Void,
    ) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::casimircontrolserver::Void>> {
        self.init_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn close_opt(
        &self,
        req: &super::casimircontrolserver::Void,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<super::casimircontrolserver::Void> {
        self.client.unary_call(&METHOD_CASIMIR_CONTROL_SERVICE_CLOSE, req, opt)
    }

    pub fn close(
        &self,
        req: &super::casimircontrolserver::Void,
    ) -> ::grpcio::Result<super::casimircontrolserver::Void> {
        self.close_opt(req, ::grpcio::CallOption::default())
    }

    pub fn close_async_opt(
        &self,
        req: &super::casimircontrolserver::Void,
        opt: ::grpcio::CallOption,
    ) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::casimircontrolserver::Void>> {
        self.client.unary_call_async(&METHOD_CASIMIR_CONTROL_SERVICE_CLOSE, req, opt)
    }

    pub fn close_async(
        &self,
        req: &super::casimircontrolserver::Void,
    ) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::casimircontrolserver::Void>> {
        self.close_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F)
    where
        F: ::std::future::Future<Output = ()> + Send + 'static,
    {
        self.client.spawn(f)
    }
}

pub trait CasimirControlService {
    fn send_apdu(
        &mut self,
        ctx: ::grpcio::RpcContext,
        _req: super::casimircontrolserver::SendApduRequest,
        sink: ::grpcio::UnarySink<super::casimircontrolserver::SendApduReply>,
    ) {
        grpcio::unimplemented_call!(ctx, sink)
    }
    fn poll_a(
        &mut self,
        ctx: ::grpcio::RpcContext,
        _req: super::casimircontrolserver::Void,
        sink: ::grpcio::UnarySink<super::casimircontrolserver::SenderId>,
    ) {
        grpcio::unimplemented_call!(ctx, sink)
    }
    fn set_radio_state(
        &mut self,
        ctx: ::grpcio::RpcContext,
        _req: super::casimircontrolserver::RadioState,
        sink: ::grpcio::UnarySink<super::casimircontrolserver::Void>,
    ) {
        grpcio::unimplemented_call!(ctx, sink)
    }
    fn set_power_level(
        &mut self,
        ctx: ::grpcio::RpcContext,
        _req: super::casimircontrolserver::PowerLevel,
        sink: ::grpcio::UnarySink<super::casimircontrolserver::Void>,
    ) {
        grpcio::unimplemented_call!(ctx, sink)
    }
    fn send_broadcast(
        &mut self,
        ctx: ::grpcio::RpcContext,
        _req: super::casimircontrolserver::SendBroadcastRequest,
        sink: ::grpcio::UnarySink<super::casimircontrolserver::SendBroadcastResponse>,
    ) {
        grpcio::unimplemented_call!(ctx, sink)
    }
    fn init(
        &mut self,
        ctx: ::grpcio::RpcContext,
        _req: super::casimircontrolserver::Void,
        sink: ::grpcio::UnarySink<super::casimircontrolserver::Void>,
    ) {
        grpcio::unimplemented_call!(ctx, sink)
    }
    fn close(
        &mut self,
        ctx: ::grpcio::RpcContext,
        _req: super::casimircontrolserver::Void,
        sink: ::grpcio::UnarySink<super::casimircontrolserver::Void>,
    ) {
        grpcio::unimplemented_call!(ctx, sink)
    }
}

pub fn create_casimir_control_service<S: CasimirControlService + Send + Clone + 'static>(
    s: S,
) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder
        .add_unary_handler(&METHOD_CASIMIR_CONTROL_SERVICE_SEND_APDU, move |ctx, req, resp| {
            instance.send_apdu(ctx, req, resp)
        });
    let mut instance = s.clone();
    builder = builder
        .add_unary_handler(&METHOD_CASIMIR_CONTROL_SERVICE_POLL_A, move |ctx, req, resp| {
            instance.poll_a(ctx, req, resp)
        });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(
        &METHOD_CASIMIR_CONTROL_SERVICE_SET_RADIO_STATE,
        move |ctx, req, resp| instance.set_radio_state(ctx, req, resp),
    );
    let mut instance = s.clone();
    builder = builder.add_unary_handler(
        &METHOD_CASIMIR_CONTROL_SERVICE_SET_POWER_LEVEL,
        move |ctx, req, resp| instance.set_power_level(ctx, req, resp),
    );
    let mut instance = s.clone();
    builder = builder.add_unary_handler(
        &METHOD_CASIMIR_CONTROL_SERVICE_SEND_BROADCAST,
        move |ctx, req, resp| instance.send_broadcast(ctx, req, resp),
    );
    let mut instance = s.clone();
    builder = builder
        .add_unary_handler(&METHOD_CASIMIR_CONTROL_SERVICE_INIT, move |ctx, req, resp| {
            instance.init(ctx, req, resp)
        });
    let mut instance = s;
    builder = builder
        .add_unary_handler(&METHOD_CASIMIR_CONTROL_SERVICE_CLOSE, move |ctx, req, resp| {
            instance.close(ctx, req, resp)
        });
    builder.build()
}
