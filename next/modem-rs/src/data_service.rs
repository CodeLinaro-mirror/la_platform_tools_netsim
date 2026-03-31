// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/data_service.rs

use std::collections::HashMap;

use crate::{
    modem::ModemImpl,
    parser::{Command, QuotedString},
    types::{ExecutionResult, HandledCommand, DEFAULT_IP_ADDRESS},
};

#[derive(Debug, Clone, Default)]
pub struct Qos {
    pub precedence: u8,
    pub delay: u8,
    pub reliability: u8,
    pub peak: u8,
    pub mean: u8,
}

#[derive(Debug, Clone)]
pub struct PdpContext {
    pub pdp_type: String,
    pub apn: String,
    pub active: bool,
    pub qos: Qos,
    pub req_qos: Qos,
    pub gprs_qos: Qos,
    pub gprs_req_qos: Qos,
}

pub struct DataService {
    pdp_contexts: HashMap<u8, PdpContext>,
}

impl DataService {
    pub fn new() -> Self {
        Self { pdp_contexts: HashMap::new() }
    }

    // --- Helper methods for external services ---

    pub fn on_update_physical_channel_configs(
        &self,
        _context: &ModemImpl,
    ) -> Vec<crate::modem::ModemEffect> {
        vec![crate::modem::ModemEffect::Response(b"+CGEV: NW PDN DEACT 1\r\n".to_vec())]
    }

    // --- Pure command handlers ---

    pub fn handle_define_pdp_context(
        &mut self,
        cid: u8,
        pdp_type: QuotedString,
        apn: QuotedString,
    ) -> ExecutionResult {
        self.pdp_contexts.insert(
            cid,
            PdpContext {
                pdp_type: String::from_utf8(pdp_type.to_vec()).unwrap_or_default(),
                apn: String::from_utf8(apn.to_vec()).unwrap_or_default(),
                active: false,
                qos: Qos::default(),
                req_qos: Qos::default(),
                gprs_qos: Qos::default(),
                gprs_req_qos: Qos::default(),
            },
        );
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_query_pdp_context(&self) -> ExecutionResult {
        let mut responses = Vec::new();
        for (cid, context) in self.pdp_contexts.iter() {
            responses.push(format!(
                "+CGDCONT: {},\"{}\",\"{}\",,0,0\r\n",
                cid, context.pdp_type, context.apn
            ));
        }
        responses.push("OK\r\n".to_string());
        ExecutionResult::Handled(HandledCommand { responses, action: None })
    }

    pub fn handle_set_quality_of_service_minimum(
        &mut self,
        cid: u8,
        precedence: u8,
        delay: u8,
        reliability: u8,
        peak: u8,
        mean: u8,
    ) -> ExecutionResult {
        if let Some(context) = self.pdp_contexts.get_mut(&cid) {
            context.qos = Qos { precedence, delay, reliability, peak, mean };
            ExecutionResult::Handled(HandledCommand::ok())
        } else {
            ExecutionResult::Handled(HandledCommand::error())
        }
    }

    pub fn handle_query_quality_of_service_minimum(&self) -> ExecutionResult {
        let mut responses = Vec::new();
        for (cid, context) in self.pdp_contexts.iter() {
            responses.push(format!(
                "+CGEQMIN: {},{},{},{},{},{}\r\n",
                cid,
                context.qos.precedence,
                context.qos.delay,
                context.qos.reliability,
                context.qos.peak,
                context.qos.mean
            ));
        }
        responses.push("OK\r\n".to_string());
        ExecutionResult::Handled(HandledCommand { responses, action: None })
    }

    pub fn handle_set_quality_of_service_requested(
        &mut self,
        cid: u8,
        precedence: u8,
        delay: u8,
        reliability: u8,
        peak: u8,
        mean: u8,
    ) -> ExecutionResult {
        if let Some(context) = self.pdp_contexts.get_mut(&cid) {
            context.req_qos = Qos { precedence, delay, reliability, peak, mean };
            ExecutionResult::Handled(HandledCommand::ok())
        } else {
            ExecutionResult::Handled(HandledCommand::error())
        }
    }

    pub fn handle_query_quality_of_service_requested(&self) -> ExecutionResult {
        let mut responses = Vec::new();
        for (cid, context) in self.pdp_contexts.iter() {
            responses.push(format!(
                "+CGEQREQ: {},{},{},{},{},{}\r\n",
                cid,
                context.req_qos.precedence,
                context.req_qos.delay,
                context.req_qos.reliability,
                context.req_qos.peak,
                context.req_qos.mean
            ));
        }
        responses.push("OK\r\n".to_string());
        ExecutionResult::Handled(HandledCommand { responses, action: None })
    }

    pub fn handle_set_quality_of_service_minimum_gprs(
        &mut self,
        cid: u8,
        precedence: u8,
        delay: u8,
        reliability: u8,
        peak: u8,
        mean: u8,
    ) -> ExecutionResult {
        if let Some(context) = self.pdp_contexts.get_mut(&cid) {
            context.gprs_qos = Qos { precedence, delay, reliability, peak, mean };
            ExecutionResult::Handled(HandledCommand::ok())
        } else {
            ExecutionResult::Handled(HandledCommand::error())
        }
    }

    pub fn handle_query_quality_of_service_minimum_gprs(&self) -> ExecutionResult {
        let mut responses = Vec::new();
        for (cid, context) in self.pdp_contexts.iter() {
            responses.push(format!(
                "+CGQMIN: {},{},{},{},{},{}\r\n",
                cid,
                context.gprs_qos.precedence,
                context.gprs_qos.delay,
                context.gprs_qos.reliability,
                context.gprs_qos.peak,
                context.gprs_qos.mean
            ));
        }
        responses.push("OK\r\n".to_string());
        ExecutionResult::Handled(HandledCommand { responses, action: None })
    }

    pub fn handle_set_quality_of_service_requested_gprs(
        &mut self,
        cid: u8,
        precedence: u8,
        delay: u8,
        reliability: u8,
        peak: u8,
        mean: u8,
    ) -> ExecutionResult {
        if let Some(context) = self.pdp_contexts.get_mut(&cid) {
            context.gprs_req_qos = Qos { precedence, delay, reliability, peak, mean };
            ExecutionResult::Handled(HandledCommand::ok())
        } else {
            ExecutionResult::Handled(HandledCommand::error())
        }
    }

    pub fn handle_query_quality_of_service_requested_gprs(&self) -> ExecutionResult {
        let mut responses = Vec::new();
        for (cid, context) in self.pdp_contexts.iter() {
            responses.push(format!(
                "+CGQREQ: {},{},{},{},{},{}\r\n",
                cid,
                context.gprs_req_qos.precedence,
                context.gprs_req_qos.delay,
                context.gprs_req_qos.reliability,
                context.gprs_req_qos.peak,
                context.gprs_req_qos.mean
            ));
        }
        responses.push("OK\r\n".to_string());
        ExecutionResult::Handled(HandledCommand { responses, action: None })
    }

    pub fn handle_set_pdp_context_activate(&mut self, cid: u8, state: u8) -> ExecutionResult {
        if let Some(context) = self.pdp_contexts.get_mut(&cid) {
            context.active = state == 1;
            ExecutionResult::Handled(HandledCommand::ok())
        } else {
            ExecutionResult::Handled(HandledCommand::error())
        }
    }

    pub fn handle_set_ps_attach(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_pdp_context_modify(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_enter_data_state(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand {
            responses: vec!["CONNECT\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_set_packet_event_reporting(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_show_pdp_address(&self, cid: u8) -> ExecutionResult {
        if let Some(context) = self.pdp_contexts.get(&cid) {
            let ip_address = if context.active { DEFAULT_IP_ADDRESS } else { "0.0.0.0" };
            let response = format!("+CGPADDR: {},\"{}\"\r\n", cid, ip_address);
            let mut handled = HandledCommand::ok();
            handled.responses.insert(0, response);
            ExecutionResult::Handled(handled)
        } else {
            ExecutionResult::Handled(HandledCommand::error())
        }
    }

    pub fn handle_read_dynamic_param(&self, cid: u8) -> ExecutionResult {
        let response = format!("+CGSCONTRDP: {}, 5, 1500, 300000, 300000, 300000, 300000\r\n", cid);
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    pub fn execute(&mut self, command: &Command) -> ExecutionResult {
        match command {
            Command::DefinePdpContext(cid, pdp_type, apn) => {
                self.handle_define_pdp_context(*cid, *pdp_type, *apn)
            }
            Command::QueryPdpContext => self.handle_query_pdp_context(),
            Command::QueryQualityOfServiceMinimum => self.handle_query_quality_of_service_minimum(),
            Command::SetQualityOfServiceMinimum(cid, prec, delay, rel, peak, mean) => {
                self.handle_set_quality_of_service_minimum(*cid, *prec, *delay, *rel, *peak, *mean)
            }
            Command::SetQualityOfServiceRequested(cid, prec, delay, rel, peak, mean) => self
                .handle_set_quality_of_service_requested(*cid, *prec, *delay, *rel, *peak, *mean),
            Command::QueryQualityOfServiceRequested => {
                self.handle_query_quality_of_service_requested()
            }
            Command::SetQualityOfServiceMinimumGprs(cid, prec, delay, rel, peak, mean) => self
                .handle_set_quality_of_service_minimum_gprs(
                    *cid, *prec, *delay, *rel, *peak, *mean,
                ),
            Command::QueryQualityOfServiceMinimumGprs => {
                self.handle_query_quality_of_service_minimum_gprs()
            }
            Command::SetQualityOfServiceRequestedGprs(cid, prec, delay, rel, peak, mean) => self
                .handle_set_quality_of_service_requested_gprs(
                    *cid, *prec, *delay, *rel, *peak, *mean,
                ),
            Command::QueryQualityOfServiceRequestedGprs => {
                self.handle_query_quality_of_service_requested_gprs()
            }
            Command::SetPdpContextActivate(cid, state) => {
                self.handle_set_pdp_context_activate(*cid, *state)
            }
            Command::SetPsAttach(_) => self.handle_set_ps_attach(),
            Command::SetPdpContextModify(_) => self.handle_set_pdp_context_modify(),
            Command::EnterDataState(_) => self.handle_enter_data_state(),
            Command::SetPacketEventReporting(_, _) => self.handle_set_packet_event_reporting(),
            Command::ShowPdpAddress(cid) => self.handle_show_pdp_address(*cid),
            Command::ReadDynamicParam(cid) => self.handle_read_dynamic_param(*cid),
            _ => ExecutionResult::Unhandled,
        }
    }
}
