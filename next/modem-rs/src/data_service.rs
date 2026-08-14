// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/data_service.rs

use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use modem_rs_derive::CommandParser;

use crate::{
    cuttlefish::read_cuttlefish_config,
    modem::ModemImpl,
    parser::QuotedString,
    types::{
        DEFAULT_DNS, DEFAULT_GATEWAY, DEFAULT_IPV4_ADDR, DEFAULT_IPV6_ADDR, DEFAULT_IPV6_DNS,
        DEFAULT_IPV6_GATEWAY, DEFAULT_IPV6_PREFIX, ExecutionResult, Parsable, PdpType,
    },
};

/// Data service AT commands.
#[derive(Debug, PartialEq, Clone, Copy, CommandParser)]
pub enum DataCommand<'a> {
    #[command(tag = "AT+CGDCONT=")]
    DefinePdpContext(
        u8,
        PdpType,
        QuotedString<'a>,
        Option<QuotedString<'a>>,
        Option<u8>,
        Option<u8>,
    ),
    #[command(tag = "AT+CGDCONT?")]
    QueryPdpContext,
    #[command(tag = "AT+CGEQMIN=")]
    SetQualityOfServiceMinimum(u8, u8, u8, u8, u8, u8),
    #[command(tag = "AT+CGEQMIN?")]
    QueryQualityOfServiceMinimum,
    #[command(tag = "AT+CGEQREQ=")]
    SetQualityOfServiceRequested(u8, u8, u8, u8, u8, u8),
    #[command(tag = "AT+CGEQREQ?")]
    QueryQualityOfServiceRequested,
    #[command(tag = "AT+CGQMIN=")]
    SetQualityOfServiceMinimumGprs(u8, u8, u8, u8, u8, u8),
    #[command(tag = "AT+CGQMIN?")]
    QueryQualityOfServiceMinimumGprs,
    #[command(tag = "AT+CGQREQ=")]
    SetQualityOfServiceRequestedGprs(u8, u8, u8, u8, u8, u8),
    #[command(tag = "AT+CGQREQ?")]
    QueryQualityOfServiceRequestedGprs,
    #[command(tag = "AT+CGACT=")]
    SetPdpContextActivate(u8, u8),
    #[command(tag = "AT+CGACT?")]
    QueryPdpContextActivate,
    #[command(tag = "AT+CGATT=")]
    SetPsAttach(u8),
    #[command(tag = "AT+CGATT?")]
    QueryPsAttach,
    #[command(tag = "AT+CGCMOD=")]
    SetPdpContextModify(u8),
    #[command(tag = "AT+CGDATA=")]
    EnterDataState(u8),
    #[command(tag = "AT+CGEREP=")]
    SetPacketEventReporting(u8, u8),
    #[command(tag = "AT+CGPADDR=")]
    ShowPdpAddress(u8),
    #[command(tag = "AT+CGCONTRDP=")]
    ReadDynamicParam(u8),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Qos {
    pub precedence: u8,
    pub delay: u8,
    pub reliability: u8,
    pub peak: u8,
    pub mean: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdpContext {
    pub pdp_type: PdpType,
    pub apn: String,
    pub active: bool,
    pub qos: Qos,
    pub req_qos: Qos,
    pub gprs_qos: Qos,
    pub gprs_req_qos: Qos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataResponse {
    PdpContexts(Vec<(u8, PdpContext)>),
    QosMinimum(Vec<(u8, Qos)>),
    QosRequested(Vec<(u8, Qos)>),
    QosMinimumGprs(Vec<(u8, Qos)>),
    QosRequestedGprs(Vec<(u8, Qos)>),
    PsAttach(bool),
    PdpContextActivate(Vec<(u8, bool)>),
    Connect,
    PdpAddress {
        cid: u8,
        ip_address: String,
    },
    DynamicParam {
        cid: u8,
        apn: String,
        ip_address: String,
        prefix: u32,
        gateway: String,
        dns: String,
    },
}

impl std::fmt::Display for DataResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataResponse::PdpContexts(contexts) => {
                for (cid, context) in contexts {
                    write!(
                        f,
                        "+CGDCONT: {cid},\"{}\",\"{}\",,0,0\r\n",
                        context.pdp_type, context.apn
                    )?;
                }
                Ok(())
            }
            DataResponse::QosMinimum(qos_list) => {
                for (cid, qos) in qos_list {
                    write!(
                        f,
                        "+CGEQMIN: {cid},{},{},{},{},{}\r\n",
                        qos.precedence, qos.delay, qos.reliability, qos.peak, qos.mean
                    )?;
                }
                Ok(())
            }
            DataResponse::QosRequested(qos_list) => {
                for (cid, qos) in qos_list {
                    write!(
                        f,
                        "+CGEQREQ: {cid},{},{},{},{},{}\r\n",
                        qos.precedence, qos.delay, qos.reliability, qos.peak, qos.mean
                    )?;
                }
                Ok(())
            }
            DataResponse::QosMinimumGprs(qos_list) => {
                for (cid, qos) in qos_list {
                    write!(
                        f,
                        "+CGQMIN: {cid},{},{},{},{},{}\r\n",
                        qos.precedence, qos.delay, qos.reliability, qos.peak, qos.mean
                    )?;
                }
                Ok(())
            }
            DataResponse::QosRequestedGprs(qos_list) => {
                for (cid, qos) in qos_list {
                    write!(
                        f,
                        "+CGQREQ: {cid},{},{},{},{},{}\r\n",
                        qos.precedence, qos.delay, qos.reliability, qos.peak, qos.mean
                    )?;
                }
                Ok(())
            }
            DataResponse::PsAttach(attached) => {
                let state = if *attached { 1 } else { 0 };
                write!(f, "+CGATT: {state}\r\n")
            }
            DataResponse::PdpContextActivate(active_list) => {
                for (cid, active) in active_list {
                    let state = if *active { 1 } else { 0 };
                    write!(f, "+CGACT: {cid},{state}\r\n")?;
                }
                Ok(())
            }
            DataResponse::Connect => {
                write!(f, "CONNECT\r\n")
            }
            DataResponse::PdpAddress { cid, ip_address } => {
                write!(f, "+CGPADDR: {cid},\"{ip_address}\"\r\n")
            }
            DataResponse::DynamicParam { cid, apn, ip_address, prefix, gateway, dns } => {
                write!(f, "+CGCONTRDP: {cid},5,\"{apn}\",{ip_address}/{prefix},{gateway},{dns}\r\n")
            }
        }
    }
}

type DataResult = Result<Option<DataResponse>, ExecutionResult>;

pub struct DataService {
    pdp_contexts: BTreeMap<u8, PdpContext>,
    ip_address: Option<IpAddr>,
    prefixlen: Option<u32>,
    gateway: Option<IpAddr>,
    dns: Option<IpAddr>,
    ps_attached: bool,
}

impl DataService {
    pub fn new(
        ip_address: Option<IpAddr>,
        prefixlen: Option<u32>,
        gateway: Option<IpAddr>,
        dns: Option<IpAddr>,
    ) -> Self {
        Self {
            pdp_contexts: BTreeMap::new(),
            ip_address,
            prefixlen,
            gateway,
            dns,
            ps_attached: true,
        }
    }

    pub fn from_env() -> Self {
        if let Some(config) = read_cuttlefish_config() {
            let ip_address = config
                .ip_address
                .parse::<IpAddr>()
                .map_err(|_err| {
                    tracing::warn!("Configured IP ({}) is invalid.", config.ip_address);
                })
                .ok();
            let gateway = config
                .gateway
                .parse::<IpAddr>()
                .map_err(|_err| {
                    tracing::warn!("Configured gateway ({}) is invalid.", config.gateway);
                })
                .ok();
            let dns = config
                .dns
                .parse::<IpAddr>()
                .map_err(|_err| {
                    tracing::warn!("Configured DNS ({}) is invalid.", config.dns);
                })
                .ok();

            Self::new(ip_address, Some(config.prefixlen), gateway, dns)
        } else {
            Self::default()
        }
    }
}

impl Default for DataService {
    fn default() -> Self {
        Self::new(None, None, None, None)
    }
}

impl DataService {
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
        pdp_type: PdpType,
        apn: QuotedString,
    ) -> DataResult {
        self.pdp_contexts.insert(
            cid,
            PdpContext {
                pdp_type,
                apn: String::from_utf8(apn.to_vec()).unwrap_or_default(),
                active: self.ps_attached, // Goldfish expects data to be auto-activated
                qos: Qos::default(),
                req_qos: Qos::default(),
                gprs_qos: Qos::default(),
                gprs_req_qos: Qos::default(),
            },
        );
        Ok(None)
    }

    pub fn handle_query_pdp_context(&self) -> DataResult {
        if self.pdp_contexts.is_empty() {
            Ok(None)
        } else {
            let mut contexts = Vec::new();
            for (cid, context) in &self.pdp_contexts {
                contexts.push((*cid, context.clone()));
            }
            Ok(Some(DataResponse::PdpContexts(contexts)))
        }
    }

    pub fn handle_set_quality_of_service_minimum(
        &mut self,
        cid: u8,
        precedence: u8,
        delay: u8,
        reliability: u8,
        peak: u8,
        mean: u8,
    ) -> DataResult {
        if let Some(context) = self.pdp_contexts.get_mut(&cid) {
            context.qos = Qos { precedence, delay, reliability, peak, mean };
            Ok(None)
        } else {
            Err(ExecutionResult::error())
        }
    }

    pub fn handle_query_quality_of_service_minimum(&self) -> DataResult {
        if self.pdp_contexts.is_empty() {
            Ok(None)
        } else {
            let mut qos_list = Vec::new();
            for (cid, context) in &self.pdp_contexts {
                qos_list.push((*cid, context.qos.clone()));
            }
            Ok(Some(DataResponse::QosMinimum(qos_list)))
        }
    }

    pub fn handle_set_quality_of_service_requested(
        &mut self,
        cid: u8,
        precedence: u8,
        delay: u8,
        reliability: u8,
        peak: u8,
        mean: u8,
    ) -> DataResult {
        if let Some(context) = self.pdp_contexts.get_mut(&cid) {
            context.req_qos = Qos { precedence, delay, reliability, peak, mean };
            Ok(None)
        } else {
            Err(ExecutionResult::error())
        }
    }

    pub fn handle_query_quality_of_service_requested(&self) -> DataResult {
        if self.pdp_contexts.is_empty() {
            Ok(None)
        } else {
            let mut qos_list = Vec::new();
            for (cid, context) in &self.pdp_contexts {
                qos_list.push((*cid, context.req_qos.clone()));
            }
            Ok(Some(DataResponse::QosRequested(qos_list)))
        }
    }

    pub fn handle_set_quality_of_service_minimum_gprs(
        &mut self,
        cid: u8,
        precedence: u8,
        delay: u8,
        reliability: u8,
        peak: u8,
        mean: u8,
    ) -> DataResult {
        if let Some(context) = self.pdp_contexts.get_mut(&cid) {
            context.gprs_qos = Qos { precedence, delay, reliability, peak, mean };
            Ok(None)
        } else {
            Err(ExecutionResult::error())
        }
    }

    pub fn handle_query_quality_of_service_minimum_gprs(&self) -> DataResult {
        if self.pdp_contexts.is_empty() {
            Ok(None)
        } else {
            let mut qos_list = Vec::new();
            for (cid, context) in &self.pdp_contexts {
                qos_list.push((*cid, context.gprs_qos.clone()));
            }
            Ok(Some(DataResponse::QosMinimumGprs(qos_list)))
        }
    }

    pub fn handle_set_quality_of_service_requested_gprs(
        &mut self,
        cid: u8,
        precedence: u8,
        delay: u8,
        reliability: u8,
        peak: u8,
        mean: u8,
    ) -> DataResult {
        if let Some(context) = self.pdp_contexts.get_mut(&cid) {
            context.gprs_req_qos = Qos { precedence, delay, reliability, peak, mean };
            Ok(None)
        } else {
            Err(ExecutionResult::error())
        }
    }

    pub fn handle_query_quality_of_service_requested_gprs(&self) -> DataResult {
        if self.pdp_contexts.is_empty() {
            Ok(None)
        } else {
            let mut qos_list = Vec::new();
            for (cid, context) in &self.pdp_contexts {
                qos_list.push((*cid, context.gprs_req_qos.clone()));
            }
            Ok(Some(DataResponse::QosRequestedGprs(qos_list)))
        }
    }

    pub fn handle_set_pdp_context_activate(&mut self, cid: u8, state: u8) -> DataResult {
        if let Some(context) = self.pdp_contexts.get_mut(&cid) {
            context.active = state == 1;
            Ok(None)
        } else {
            Err(ExecutionResult::error())
        }
    }

    pub fn handle_set_ps_attach(&mut self, state: u8) -> DataResult {
        self.ps_attached = state == 1;
        if !self.ps_attached {
            for context in self.pdp_contexts.values_mut() {
                context.active = false;
            }
        }
        Ok(None)
    }

    pub fn handle_query_ps_attach(&self) -> DataResult {
        Ok(Some(DataResponse::PsAttach(self.ps_attached)))
    }

    pub fn handle_query_pdp_context_activate(&self) -> DataResult {
        if self.pdp_contexts.is_empty() {
            Ok(None)
        } else {
            let mut active_list = Vec::new();
            for (cid, context) in &self.pdp_contexts {
                active_list.push((*cid, context.active));
            }
            Ok(Some(DataResponse::PdpContextActivate(active_list)))
        }
    }

    pub fn handle_set_pdp_context_modify(&self, cid: u8) -> DataResult {
        if self.pdp_contexts.contains_key(&cid) { Ok(None) } else { Err(ExecutionResult::error()) }
    }

    pub fn handle_enter_data_state(&self, cid: u8) -> DataResult {
        match self.pdp_contexts.get(&cid) {
            Some(context) if context.active => Ok(Some(DataResponse::Connect)),
            _ => Err(ExecutionResult::error()),
        }
    }

    pub fn handle_set_packet_event_reporting(&self) -> DataResult {
        Ok(None)
    }

    fn get_ipv4_address(&self, cid: u8, base_ip: Ipv4Addr) -> Ipv4Addr {
        let octets = base_ip.octets();
        if cid <= 1 {
            base_ip
        } else {
            let raw_last_octet = (octets[3] as u32) + (cid.saturating_sub(1) as u32);
            if raw_last_octet > 254 {
                tracing::warn!(
                    "IP address last octet saturated to 254 for cid {cid}. Base IP: {base_ip}, calculated octet: {raw_last_octet}"
                );
            }
            let last_octet = std::cmp::min(raw_last_octet, 254) as u8;
            Ipv4Addr::new(octets[0], octets[1], octets[2], last_octet)
        }
    }

    fn get_ipv6_address(&self, cid: u8, base_ip: Ipv6Addr) -> Ipv6Addr {
        let mut segments = base_ip.segments();
        let offset = cid.saturating_sub(1) as u16;
        segments[7] = segments[7].saturating_add(offset);
        Ipv6Addr::from(segments)
    }

    fn get_ip_address(&self, cid: u8, pdp_type: &PdpType) -> IpAddr {
        match self.ip_address {
            Some(IpAddr::V4(ipv4)) => {
                if *pdp_type == PdpType::Ipv6 {
                    tracing::warn!(
                        "Configured IP ({}) is IPv4, but IPv6 was requested. Using IPv4 anyway.",
                        ipv4
                    );
                }
                IpAddr::V4(self.get_ipv4_address(cid, ipv4))
            }
            Some(IpAddr::V6(ipv6)) => {
                if *pdp_type != PdpType::Ipv6 {
                    tracing::warn!(
                        "Configured IP ({}) is IPv6, but IPv4 was requested. Using IPv6 anyway.",
                        ipv6
                    );
                }
                IpAddr::V6(self.get_ipv6_address(cid, ipv6))
            }
            None => self.get_default_ip(cid, pdp_type),
        }
    }

    fn get_default_ip(&self, cid: u8, pdp_type: &PdpType) -> IpAddr {
        if *pdp_type == PdpType::Ipv6 {
            IpAddr::V6(self.get_ipv6_address(cid, DEFAULT_IPV6_ADDR))
        } else {
            if cid <= 1 {
                IpAddr::V4(DEFAULT_IPV4_ADDR)
            } else {
                let octets = DEFAULT_IPV4_ADDR.octets();
                let last_octet = std::cmp::min(98u8.saturating_add(cid), 254);
                IpAddr::V4(Ipv4Addr::new(octets[0], octets[1], octets[2], last_octet))
            }
        }
    }

    fn get_gateway(&self, resolved_ip: &IpAddr) -> IpAddr {
        match (resolved_ip, self.gateway) {
            (IpAddr::V4(_), Some(IpAddr::V4(gw))) => IpAddr::V4(gw),
            (IpAddr::V6(_), Some(IpAddr::V6(gw))) => IpAddr::V6(gw),
            (IpAddr::V4(_), _) => IpAddr::V4(DEFAULT_GATEWAY),
            (IpAddr::V6(_), _) => IpAddr::V6(DEFAULT_IPV6_GATEWAY),
        }
    }

    fn get_dns(&self, resolved_ip: &IpAddr) -> IpAddr {
        match (resolved_ip, self.dns) {
            (IpAddr::V4(_), Some(IpAddr::V4(dns))) => IpAddr::V4(dns),
            (IpAddr::V6(_), Some(IpAddr::V6(dns))) => IpAddr::V6(dns),
            (IpAddr::V4(_), _) => IpAddr::V4(DEFAULT_DNS),
            (IpAddr::V6(_), _) => IpAddr::V6(DEFAULT_IPV6_DNS),
        }
    }

    pub fn handle_show_pdp_address(&self, cid: u8) -> DataResult {
        if let Some(context) = self.pdp_contexts.get(&cid) {
            if context.active {
                let requested_type =
                    if context.pdp_type == PdpType::Ipv6 { PdpType::Ipv6 } else { PdpType::Ip };
                let ip_address = self.get_ip_address(cid, &requested_type).to_string();
                Ok(Some(DataResponse::PdpAddress { cid, ip_address }))
            } else {
                Err(ExecutionResult::error())
            }
        } else {
            Err(ExecutionResult::error())
        }
    }

    pub fn handle_read_dynamic_param(&self, cid: u8) -> DataResult {
        if let Some(context) = self.pdp_contexts.get(&cid) {
            if context.active {
                let apn = context.apn.clone();
                // TODO(b/542980136): Support true dual-stack (IPV4V6) by returning both IPv4
                // and IPv6 parameters. Currently, we only enable IPv6 if it is
                // purely IPV6 to avoid breaking IPv4 compatibility on IPV4V6.
                let is_ipv6 = context.pdp_type == PdpType::Ipv6;
                let requested_type = if is_ipv6 { PdpType::Ipv6 } else { PdpType::Ip };

                let ip_address = self.get_ip_address(cid, &requested_type);
                let gateway = self.get_gateway(&ip_address);
                let dns = self.get_dns(&ip_address);

                let (ip_str, gw_str, dns_str, prefix) = if ip_address.is_ipv6() {
                    (
                        ip_address.to_string(),
                        gateway.to_string(),
                        dns.to_string(),
                        self.prefixlen
                            .filter(|&p| (Ipv4Addr::BITS + 1..=Ipv6Addr::BITS).contains(&p))
                            .unwrap_or(DEFAULT_IPV6_PREFIX),
                    )
                } else {
                    (
                        ip_address.to_string(),
                        gateway.to_string(),
                        dns.to_string(),
                        self.prefixlen.unwrap_or(24),
                    )
                };
                Ok(Some(DataResponse::DynamicParam {
                    cid,
                    apn,
                    ip_address: ip_str,
                    prefix,
                    gateway: gw_str,
                    dns: dns_str,
                }))
            } else {
                Err(ExecutionResult::error())
            }
        } else {
            Err(ExecutionResult::error())
        }
    }

    pub fn handle_gprs_dial(&mut self, number: &[u8]) -> DataResult {
        match parse_cid_from_gprs_dial(number) {
            Ok(cid) => {
                if let Some(context) = self.pdp_contexts.get_mut(&cid) {
                    context.active = true;
                    Ok(Some(DataResponse::Connect))
                } else {
                    Err(ExecutionResult::error())
                }
            }
            Err(_) => Err(ExecutionResult::error()),
        }
    }

    pub fn execute<'a>(&mut self, command: &DataCommand<'a>) -> ExecutionResult {
        let result: DataResult = match command {
            DataCommand::DefinePdpContext(cid, pdp_type, apn, ..) => {
                self.handle_define_pdp_context(*cid, *pdp_type, *apn)
            }
            DataCommand::QueryPdpContext => self.handle_query_pdp_context(),
            DataCommand::QueryQualityOfServiceMinimum => {
                self.handle_query_quality_of_service_minimum()
            }
            DataCommand::SetQualityOfServiceMinimum(cid, prec, delay, rel, peak, mean) => {
                self.handle_set_quality_of_service_minimum(*cid, *prec, *delay, *rel, *peak, *mean)
            }
            DataCommand::SetQualityOfServiceRequested(cid, prec, delay, rel, peak, mean) => self
                .handle_set_quality_of_service_requested(*cid, *prec, *delay, *rel, *peak, *mean),
            DataCommand::QueryQualityOfServiceRequested => {
                self.handle_query_quality_of_service_requested()
            }
            DataCommand::SetQualityOfServiceMinimumGprs(cid, prec, delay, rel, peak, mean) => self
                .handle_set_quality_of_service_minimum_gprs(
                    *cid, *prec, *delay, *rel, *peak, *mean,
                ),
            DataCommand::QueryQualityOfServiceMinimumGprs => {
                self.handle_query_quality_of_service_minimum_gprs()
            }
            DataCommand::SetQualityOfServiceRequestedGprs(cid, prec, delay, rel, peak, mean) => {
                self.handle_set_quality_of_service_requested_gprs(
                    *cid, *prec, *delay, *rel, *peak, *mean,
                )
            }
            DataCommand::QueryQualityOfServiceRequestedGprs => {
                self.handle_query_quality_of_service_requested_gprs()
            }
            DataCommand::SetPdpContextActivate(state, cid) => {
                // Compatibility hack for legacy Goldfish/Reference RIL.
                // It sends AT+CGACT using non-standard <cid>,<state> format.
                // We detect this by checking if the parsed state is > 1 (which means it's
                // actually the CID) or if the parsed CID is 0 (which means it's the state 0).
                let (real_cid, real_state) =
                    if *state > 1 || *cid == 0 { (*state, *cid) } else { (*cid, *state) };
                self.handle_set_pdp_context_activate(real_cid, real_state)
            }
            DataCommand::QueryPdpContextActivate => self.handle_query_pdp_context_activate(),
            DataCommand::SetPsAttach(state) => self.handle_set_ps_attach(*state),
            DataCommand::QueryPsAttach => self.handle_query_ps_attach(),
            DataCommand::SetPdpContextModify(cid) => self.handle_set_pdp_context_modify(*cid),
            DataCommand::EnterDataState(cid) => self.handle_enter_data_state(*cid),
            DataCommand::SetPacketEventReporting(_, _) => self.handle_set_packet_event_reporting(),
            DataCommand::ShowPdpAddress(cid) => self.handle_show_pdp_address(*cid),
            DataCommand::ReadDynamicParam(cid) => self.handle_read_dynamic_param(*cid),
        };
        result.into()
    }
}

fn parse_cid_from_gprs_dial(number: &[u8]) -> Result<u8, ()> {
    let trimmed = number.strip_suffix(b"#").ok_or(())?;
    let parts: Vec<&[u8]> = trimmed.split(|&b| b == b'*').collect();

    // Expecting a format like *99, *99*<cid>, or *99***<cid> (at most 5 segments)
    if parts.len() < 2 || parts.len() > 5 || !parts[0].is_empty() || parts[1] != b"99" {
        return Err(());
    }

    // Case 1: *99# (parts are ["", "99"])
    if parts.len() == 2 {
        return Ok(1);
    }

    // Case 2: *99*<cid># or *99***<cid># (parts.len() > 2)
    let last_part = parts.last().ok_or(())?;
    if last_part.is_empty() {
        return Err(());
    }
    let cid_str = std::str::from_utf8(last_part).map_err(|_err| ())?;
    let cid = cid_str.parse::<u8>().map_err(|_err| ())?;
    Ok(cid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parser::QuotedString, types::Response};

    #[test]
    fn test_data_service_dial_direct() {
        let mut service = DataService::default();
        let res = service.handle_define_pdp_context(1, PdpType::Ip, QuotedString(b"test"));
        assert!(res.is_ok());

        // Dial
        let res: ExecutionResult = service.handle_gprs_dial(b"*99***1#").into();
        if let ExecutionResult::Success(handled) = res {
            assert_eq!(handled.responses, vec![Response::Data(DataResponse::Connect)]);
        } else {
            panic!("Expected Success");
        }
    }

    #[test]
    fn test_data_service_dial_malformed() {
        let mut service = DataService::default();
        let res = service.handle_define_pdp_context(1, PdpType::Ip, QuotedString(b"test"));
        assert!(res.is_ok());

        // Dial malformed alphanumeric CID
        let res: ExecutionResult = service.handle_gprs_dial(b"*99*abc#").into();
        assert!(matches!(res, ExecutionResult::Error { .. }));

        // Dial empty trailing CID
        let res: ExecutionResult = service.handle_gprs_dial(b"*99*#").into();
        assert!(matches!(res, ExecutionResult::Error { .. }));
    }

    #[test]
    fn test_pdp_context_auto_activation() {
        let mut service = DataService::default();
        let _ = service.handle_define_pdp_context(1, PdpType::Ip, QuotedString(b""));
        assert!(service.pdp_contexts.get(&1).unwrap().active);
    }

    #[test]
    fn test_pdp_context_no_auto_activation_when_detached() {
        let mut service = DataService::default();
        let _ = service.handle_set_ps_attach(0);
        let _ = service.handle_define_pdp_context(1, PdpType::Ip, QuotedString(b""));
        assert!(!service.pdp_contexts.get(&1).unwrap().active);
    }

    #[test]
    fn test_cuttlefish_config_parsing() {
        use std::io::Write;
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        let config_json = r#"{
            "instances": {
                "1": {
                    "ril_ipaddr": "192.168.97.2",
                    "ril_prefixlen": 30,
                    "ril_gateway": "192.168.97.1",
                    "ril_dns": "8.8.8.8"
                }
            }
        }"#;
        temp_file.write_all(config_json.as_bytes()).unwrap();

        let config_path_str = temp_file.path().to_str().unwrap();
        let config =
            crate::cuttlefish::read_cuttlefish_config_with_params(config_path_str, "1").unwrap();

        let service = DataService::new(
            config.ip_address.parse::<IpAddr>().ok(),
            Some(config.prefixlen),
            config.gateway.parse::<IpAddr>().ok(),
            config.dns.parse::<IpAddr>().ok(),
        );
        assert_eq!(service.ip_address, Some(IpAddr::V4(Ipv4Addr::new(192, 168, 97, 2))));
        assert_eq!(service.prefixlen, Some(30));
        assert_eq!(service.gateway, Some(IpAddr::V4(Ipv4Addr::new(192, 168, 97, 1))));
        assert_eq!(service.dns, Some(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));

        // Test fallback IP generation
        assert_eq!(
            service.get_ip_address(1, &PdpType::Ip),
            IpAddr::V4(Ipv4Addr::new(192, 168, 97, 2))
        );
        assert_eq!(
            service.get_ip_address(2, &PdpType::Ip),
            IpAddr::V4(Ipv4Addr::new(192, 168, 97, 3))
        );
        assert_eq!(
            service.get_ip_address(255, &PdpType::Ip),
            IpAddr::V4(Ipv4Addr::new(192, 168, 97, 254))
        );

        // Test IPv6 request on Cuttlefish (warns and returns the configured IPv4
        // address anyway)
        assert_eq!(
            service.get_ip_address(1, &PdpType::Ipv6),
            IpAddr::V4(Ipv4Addr::new(192, 168, 97, 2))
        );

        // Test Goldfish path (no config IP)
        let goldfish_service = DataService::new(None, None, None, None);
        assert_eq!(
            goldfish_service.get_ip_address(1, &PdpType::Ipv6),
            IpAddr::V6(Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 0x15))
        );
        assert_eq!(
            goldfish_service.get_ip_address(2, &PdpType::Ipv6),
            IpAddr::V6(Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 0x16))
        );
        assert_eq!(
            goldfish_service.get_ip_address(1, &PdpType::Ip),
            IpAddr::V4(Ipv4Addr::new(10, 0, 2, 15))
        );
        assert_eq!(
            goldfish_service.get_ip_address(2, &PdpType::Ip),
            IpAddr::V4(Ipv4Addr::new(10, 0, 2, 100))
        ); // Goldfish jump
    }

    #[test]
    fn test_dynamic_param_prefix_filtering() {
        // Test IPv6 context ignores small IPv4 prefixlen (e.g. 30) and defaults to
        // DEFAULT_IPV6_PREFIX (64)
        let mut service = DataService::new(None, Some(30), None, None);
        service.pdp_contexts.insert(
            1,
            PdpContext {
                pdp_type: PdpType::Ipv6,
                apn: "test.apn".to_string(),
                active: true,
                qos: Qos::default(),
                req_qos: Qos::default(),
                gprs_qos: Qos::default(),
                gprs_req_qos: Qos::default(),
            },
        );

        let res = service.handle_read_dynamic_param(1).unwrap();
        match res {
            Some(DataResponse::DynamicParam { prefix, .. }) => {
                assert_eq!(prefix, DEFAULT_IPV6_PREFIX);
            }
            _ => panic!("Expected DynamicParam response"),
        }

        // Test with valid IPv6 prefixlen (e.g. 48)
        let mut service_v6_prefix = DataService::new(
            Some(IpAddr::V6(Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 0x15))),
            Some(48),
            Some(IpAddr::V6(Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 2))),
            Some(IpAddr::V6(Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 3))),
        );
        service_v6_prefix.pdp_contexts.insert(
            1,
            PdpContext {
                pdp_type: PdpType::Ipv6,
                apn: "test.apn".to_string(),
                active: true,
                qos: Qos::default(),
                req_qos: Qos::default(),
                gprs_qos: Qos::default(),
                gprs_req_qos: Qos::default(),
            },
        );
        let res_v6 = service_v6_prefix.handle_read_dynamic_param(1).unwrap();
        match res_v6 {
            Some(DataResponse::DynamicParam { prefix, .. }) => {
                assert_eq!(prefix, 48);
            }
            _ => panic!("Expected DynamicParam response"),
        }
    }
}
