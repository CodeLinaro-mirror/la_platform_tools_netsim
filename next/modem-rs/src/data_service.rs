// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/data_service.rs

use std::collections::BTreeMap;

use crate::{
    cuttlefish::read_cuttlefish_config,
    modem::ModemImpl,
    parser::{Command, QuotedString},
    types::{DEFAULT_DNS, DEFAULT_GATEWAY, ExecutionResult},
};

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
    pub pdp_type: String,
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
                        "+CGDCONT: {},\"{}\",\"{}\",,0,0\r\n",
                        cid, context.pdp_type, context.apn
                    )?;
                }
                Ok(())
            }
            DataResponse::QosMinimum(qos_list) => {
                for (cid, qos) in qos_list {
                    write!(
                        f,
                        "+CGEQMIN: {},{},{},{},{},{}\r\n",
                        cid, qos.precedence, qos.delay, qos.reliability, qos.peak, qos.mean
                    )?;
                }
                Ok(())
            }
            DataResponse::QosRequested(qos_list) => {
                for (cid, qos) in qos_list {
                    write!(
                        f,
                        "+CGEQREQ: {},{},{},{},{},{}\r\n",
                        cid, qos.precedence, qos.delay, qos.reliability, qos.peak, qos.mean
                    )?;
                }
                Ok(())
            }
            DataResponse::QosMinimumGprs(qos_list) => {
                for (cid, qos) in qos_list {
                    write!(
                        f,
                        "+CGQMIN: {},{},{},{},{},{}\r\n",
                        cid, qos.precedence, qos.delay, qos.reliability, qos.peak, qos.mean
                    )?;
                }
                Ok(())
            }
            DataResponse::QosRequestedGprs(qos_list) => {
                for (cid, qos) in qos_list {
                    write!(
                        f,
                        "+CGQREQ: {},{},{},{},{},{}\r\n",
                        cid, qos.precedence, qos.delay, qos.reliability, qos.peak, qos.mean
                    )?;
                }
                Ok(())
            }
            DataResponse::PsAttach(attached) => {
                let state = if *attached { 1 } else { 0 };
                write!(f, "+CGATT: {}\r\n", state)
            }
            DataResponse::PdpContextActivate(active_list) => {
                for (cid, active) in active_list {
                    let state = if *active { 1 } else { 0 };
                    write!(f, "+CGACT: {},{}\r\n", cid, state)?;
                }
                Ok(())
            }
            DataResponse::Connect => {
                write!(f, "CONNECT\r\n")
            }
            DataResponse::PdpAddress { cid, ip_address } => {
                write!(f, "+CGPADDR: {},\"{}\"\r\n", cid, ip_address)
            }
            DataResponse::DynamicParam { cid, apn, ip_address, prefix, gateway, dns } => {
                write!(
                    f,
                    "+CGCONTRDP: {},5,\"{}\",{},{},{}\r\n",
                    cid,
                    apn,
                    format_args!("{}/{}", ip_address, prefix),
                    gateway,
                    dns
                )
            }
        }
    }
}

type DataResult = Result<Option<DataResponse>, ExecutionResult>;

pub struct DataService {
    pdp_contexts: BTreeMap<u8, PdpContext>,
    ip_address: Option<String>,
    prefixlen: Option<u32>,
    gateway: Option<String>,
    dns: Option<String>,
    ps_attached: bool,
}

impl DataService {
    pub fn new(
        ip_address: Option<String>,
        prefixlen: Option<u32>,
        gateway: Option<String>,
        dns: Option<String>,
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
            Self::new(
                Some(config.ip_address),
                Some(config.prefixlen),
                Some(config.gateway),
                Some(config.dns),
            )
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
        pdp_type: QuotedString,
        apn: QuotedString,
    ) -> DataResult {
        self.pdp_contexts.insert(
            cid,
            PdpContext {
                pdp_type: String::from_utf8(pdp_type.to_vec()).unwrap_or_default(),
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

    fn get_ip_address(&self, cid: u8) -> String {
        if cid <= 1 {
            self.ip_address.clone().unwrap_or_else(|| "10.0.2.15".to_string())
        } else {
            // If a custom base IP is configured (e.g., from Cuttlefish), assign IPs
            // sequentially (base, base+1, base+2...). Otherwise, fall back to
            // the legacy Goldfish RIL behavior which uses 10.0.2.15 for cid=1,
            // and 10.0.2.100+ for cid > 1.
            if let Some(ip) =
                self.ip_address.as_ref().and_then(|s| s.parse::<std::net::Ipv4Addr>().ok())
            {
                let octets = ip.octets();
                let raw_last_octet = (octets[3] as u32) + (cid.saturating_sub(1) as u32);
                if raw_last_octet > 254 {
                    tracing::warn!(
                        "IP address last octet saturated to 254 for cid {cid}. Base IP: {}, calculated octet: {raw_last_octet}",
                        self.ip_address.as_ref().unwrap()
                    );
                }
                let last_octet = std::cmp::min(raw_last_octet, 254) as u8;
                return std::net::Ipv4Addr::new(octets[0], octets[1], octets[2], last_octet)
                    .to_string();
            }
            let last_octet = std::cmp::min(98u8.saturating_add(cid), 254);
            format!("10.0.2.{last_octet}")
        }
    }

    pub fn handle_show_pdp_address(&self, cid: u8) -> DataResult {
        if let Some(context) = self.pdp_contexts.get(&cid) {
            let ip_address =
                if context.active { self.get_ip_address(cid) } else { "0.0.0.0".to_string() };
            Ok(Some(DataResponse::PdpAddress { cid, ip_address }))
        } else {
            Err(ExecutionResult::error())
        }
    }

    pub fn handle_read_dynamic_param(&self, cid: u8) -> DataResult {
        if let Some(context) = self.pdp_contexts.get(&cid) {
            if context.active {
                let ip_address = self.get_ip_address(cid);
                let apn = context.apn.clone();
                let gateway = self.gateway.clone().unwrap_or_else(|| DEFAULT_GATEWAY.to_string());
                let dns = self.dns.clone().unwrap_or_else(|| DEFAULT_DNS.to_string());
                let prefix = self.prefixlen.unwrap_or(24);
                Ok(Some(DataResponse::DynamicParam { cid, apn, ip_address, prefix, gateway, dns }))
            } else {
                Err(ExecutionResult::error())
            }
        } else {
            Err(ExecutionResult::error())
        }
    }

    pub fn execute(&mut self, command: &Command) -> ExecutionResult {
        let result: DataResult = match command {
            Command::DefinePdpContext(cid, pdp_type, apn, ..) => {
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
            Command::SetPdpContextActivate(state, cid) => {
                // Compatibility hack for legacy Goldfish/Reference RIL.
                // It sends AT+CGACT using non-standard <cid>,<state> format.
                // We detect this by checking if the parsed state is > 1 (which means it's
                // actually the CID) or if the parsed CID is 0 (which means it's the state 0).
                let (real_cid, real_state) =
                    if *state > 1 || *cid == 0 { (*state, *cid) } else { (*cid, *state) };
                self.handle_set_pdp_context_activate(real_cid, real_state)
            }
            Command::QueryPdpContextActivate => self.handle_query_pdp_context_activate(),
            Command::SetPsAttach(state) => self.handle_set_ps_attach(*state),
            Command::QueryPsAttach => self.handle_query_ps_attach(),
            Command::SetPdpContextModify(cid) => self.handle_set_pdp_context_modify(*cid),
            Command::EnterDataState(cid) => self.handle_enter_data_state(*cid),
            Command::SetPacketEventReporting(_, _) => self.handle_set_packet_event_reporting(),
            Command::ShowPdpAddress(cid) => self.handle_show_pdp_address(*cid),
            Command::ReadDynamicParam(cid) => self.handle_read_dynamic_param(*cid),
            Command::Dial(number) => {
                if crate::constants::is_gprs_dial(number) {
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
                } else {
                    Err(ExecutionResult::Unhandled)
                }
            }
            _ => Err(ExecutionResult::Unhandled),
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
        let res = service.handle_define_pdp_context(1, QuotedString(b"IP"), QuotedString(b"test"));
        assert!(res.is_ok());

        // Dial
        let res = service.execute(&Command::Dial(b"*99***1#"));
        if let ExecutionResult::Success(handled) = res {
            assert_eq!(handled.responses, vec![Response::Data(DataResponse::Connect)]);
        } else {
            panic!("Expected Success");
        }
    }

    #[test]
    fn test_data_service_dial_malformed() {
        let mut service = DataService::default();
        let res = service.handle_define_pdp_context(1, QuotedString(b"IP"), QuotedString(b"test"));
        assert!(res.is_ok());

        // Dial malformed alphanumeric CID
        let res = service.execute(&Command::Dial(b"*99*abc#"));
        assert!(matches!(res, ExecutionResult::Error { .. }));

        // Dial empty trailing CID
        let res = service.execute(&Command::Dial(b"*99*#"));
        assert!(matches!(res, ExecutionResult::Error { .. }));
    }

    #[test]
    fn test_pdp_context_auto_activation() {
        let mut service = DataService::default();
        let _ = service.handle_define_pdp_context(1, QuotedString(b""), QuotedString(b""));
        assert!(service.pdp_contexts.get(&1).unwrap().active);
    }

    #[test]
    fn test_pdp_context_no_auto_activation_when_detached() {
        let mut service = DataService::default();
        let _ = service.handle_set_ps_attach(0);
        let _ = service.handle_define_pdp_context(1, QuotedString(b""), QuotedString(b""));
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
            Some(config.ip_address),
            Some(config.prefixlen),
            Some(config.gateway),
            Some(config.dns),
        );
        assert_eq!(service.ip_address, Some("192.168.97.2".to_string()));
        assert_eq!(service.prefixlen, Some(30));
        assert_eq!(service.gateway, Some("192.168.97.1".to_string()));
        assert_eq!(service.dns, Some("8.8.8.8".to_string()));

        // Test fallback IP generation
        assert_eq!(service.get_ip_address(1), "192.168.97.2");
        assert_eq!(service.get_ip_address(2), "192.168.97.3");
    }
}
