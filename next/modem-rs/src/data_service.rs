// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/data_service.rs

use std::{collections::BTreeMap, net::IpAddr};

use modem_rs_derive::CommandParser;
use netsim_model::CellNetworkConfig;
use nom::IResult;

use crate::{
    modem::ModemImpl,
    parser::QuotedString,
    types::{
        CmeError, ExecutionResult, Layer2Protocol, PacketEventReportingMode, Parsable,
        PdpContextActivateArgs, PdpType,
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
    SetQualityOfServiceMinimum(u8, Qos),
    #[command(tag = "AT+CGEQMIN?")]
    QueryQualityOfServiceMinimum,
    #[command(tag = "AT+CGEQREQ=")]
    SetQualityOfServiceRequested(u8, Qos),
    #[command(tag = "AT+CGEQREQ?")]
    QueryQualityOfServiceRequested,
    #[command(tag = "AT+CGQMIN=")]
    SetQualityOfServiceMinimumGprs(u8, Qos),
    #[command(tag = "AT+CGQMIN?")]
    QueryQualityOfServiceMinimumGprs,
    #[command(tag = "AT+CGQREQ=")]
    SetQualityOfServiceRequestedGprs(u8, Qos),
    #[command(tag = "AT+CGQREQ?")]
    QueryQualityOfServiceRequestedGprs,
    #[command(tag = "AT+CGACT=")]
    SetPdpContextActivate(PdpContextActivateArgs),
    #[command(tag = "AT+CGACT?")]
    QueryPdpContextActivate,
    #[command(tag = "AT+CGATT=")]
    SetPsAttach(bool),
    #[command(tag = "AT+CGATT?")]
    QueryPsAttach,
    #[command(tag = "AT+CGCMOD=")]
    SetPdpContextModify(u8),
    #[command(tag = "AT+CGDATA=")]
    EnterDataState(Option<QuotedString<'a>>, Option<u8>),
    #[command(tag = "AT+CGEREP=")]
    SetPacketEventReporting(PacketEventReportingMode, Option<bool>),
    #[command(tag = "AT+CGPADDR=")]
    ShowPdpAddress(u8),
    #[command(tag = "AT+CGCONTRDP=")]
    ReadDynamicParam(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QosType {
    Minimum,
    Requested,
    MinimumGprs,
    RequestedGprs,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Qos {
    pub precedence: u8,
    pub delay: u8,
    pub reliability: u8,
    pub peak: u8,
    pub mean: u8,
}

impl Qos {
    pub const fn new(precedence: u8, delay: u8, reliability: u8, peak: u8, mean: u8) -> Self {
        Self { precedence, delay, reliability, peak, mean }
    }
}

impl<'a> Parsable<'a> for Qos {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        use nom::bytes::complete::tag;
        let (input, precedence) = u8::parse(input)?;
        let (input, _) = tag(b",")(input)?;
        let (input, delay) = u8::parse(input)?;
        let (input, _) = tag(b",")(input)?;
        let (input, reliability) = u8::parse(input)?;
        let (input, _) = tag(b",")(input)?;
        let (input, peak) = u8::parse(input)?;
        let (input, _) = tag(b",")(input)?;
        let (input, mean) = u8::parse(input)?;
        Ok((input, Qos::new(precedence, delay, reliability, peak, mean)))
    }
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

impl PdpContext {
    fn qos_mut(&mut self, qos_type: QosType) -> &mut Qos {
        match qos_type {
            QosType::Minimum => &mut self.qos,
            QosType::Requested => &mut self.req_qos,
            QosType::MinimumGprs => &mut self.gprs_qos,
            QosType::RequestedGprs => &mut self.gprs_req_qos,
        }
    }

    fn qos(&self, qos_type: QosType) -> &Qos {
        match qos_type {
            QosType::Minimum => &self.qos,
            QosType::Requested => &self.req_qos,
            QosType::MinimumGprs => &self.gprs_qos,
            QosType::RequestedGprs => &self.gprs_req_qos,
        }
    }
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
        ip_address: IpAddr,
    },
    DynamicParam {
        cid: u8,
        apn: String,
        ip_address: IpAddr,
        prefix: u8,
        gateway: IpAddr,
        dns: IpAddr,
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
                write!(
                    f,
                    "+CGCONTRDP: {cid},5,\"{apn}\",\"{ip_address}/{prefix}\",\"{gateway}\",\"{dns}\"\r\n"
                )
            }
        }
    }
}

type DataResult = Result<Option<DataResponse>, ExecutionResult>;

pub struct DataService {
    pdp_contexts: BTreeMap<u8, PdpContext>,
    network_configs: Vec<CellNetworkConfig>,
    ps_attached: bool,
}

impl DataService {
    pub fn new(network_configs: Vec<CellNetworkConfig>) -> Self {
        Self { pdp_contexts: BTreeMap::new(), network_configs, ps_attached: true }
    }

    /// Selects the configuration to use for a PDP type.
    ///
    /// Preference-ordered: `IPV4V6` selects the first matching entry.
    pub fn find_network_config(&self, pdp_type: &PdpType) -> Option<&CellNetworkConfig> {
        match pdp_type {
            PdpType::Ip => self.network_configs.iter().find(|cfg| cfg.ip_address.is_ipv4()),
            PdpType::Ipv6 => self.network_configs.iter().find(|cfg| cfg.ip_address.is_ipv6()),
            PdpType::Ipv4v6 => {
                // TODO(b/542980136): Radio HAL only accepts a single IP in +CGCONTRDP;
                // return the first config to respect the configurator's preference order.
                self.network_configs.first()
            }
            PdpType::Ppp | PdpType::NonIp | PdpType::Cell => None,
        }
    }

    /// Updates network configs for future data calls.
    ///
    /// Active calls retain their existing addresses until reactivated.
    /// Do not signal updates via `+CGEV`: the Goldfish/Cuttlefish parser lacks
    /// `CGEV` support, causing channel teardown (b/562098771).
    pub fn update_network_configs(&mut self, configs: Vec<CellNetworkConfig>) {
        self.network_configs = configs;
    }

    pub fn network_configs(&self) -> &[CellNetworkConfig] {
        &self.network_configs
    }
}

impl Default for DataService {
    fn default() -> Self {
        Self::new(Vec::new())
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
                apn: apn.as_str().to_string(),
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

    pub fn handle_set_qos(&mut self, qos_type: QosType, cid: u8, qos: Qos) -> DataResult {
        if let Some(context) = self.pdp_contexts.get_mut(&cid) {
            *context.qos_mut(qos_type) = qos;
            Ok(None)
        } else {
            Err(ExecutionResult::error())
        }
    }

    pub fn handle_query_qos(&self, qos_type: QosType) -> DataResult {
        if self.pdp_contexts.is_empty() {
            Ok(None)
        } else {
            let qos_list = self
                .pdp_contexts
                .iter()
                .map(|(cid, context)| (*cid, *context.qos(qos_type)))
                .collect();
            let response = match qos_type {
                QosType::Minimum => DataResponse::QosMinimum(qos_list),
                QosType::Requested => DataResponse::QosRequested(qos_list),
                QosType::MinimumGprs => DataResponse::QosMinimumGprs(qos_list),
                QosType::RequestedGprs => DataResponse::QosRequestedGprs(qos_list),
            };
            Ok(Some(response))
        }
    }

    pub fn handle_set_pdp_context_activate(&mut self, cid: u8, state: bool) -> DataResult {
        if let Some(context) = self.pdp_contexts.get_mut(&cid) {
            context.active = state;
            Ok(None)
        } else {
            Err(ExecutionResult::error())
        }
    }

    pub fn deactivate_all(&mut self) {
        for context in self.pdp_contexts.values_mut() {
            context.active = false;
        }
    }

    pub fn handle_set_ps_attach(&mut self, state: bool) -> DataResult {
        self.ps_attached = state;
        if !self.ps_attached {
            self.deactivate_all();
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

    pub fn handle_enter_data_state(
        &self,
        l2p: Option<QuotedString<'_>>,
        cid: Option<u8>,
    ) -> DataResult {
        if l2p.is_some_and(|l| l.as_str().parse::<Layer2Protocol>().is_err()) {
            return Err(ExecutionResult::error());
        }
        let cid = cid.unwrap_or(1);
        match self.pdp_contexts.get(&cid) {
            Some(context) if context.active => Ok(Some(DataResponse::Connect)),
            _ => Err(ExecutionResult::error()),
        }
    }

    pub fn handle_set_packet_event_reporting(&self) -> DataResult {
        Ok(None)
    }

    /// Returns the assigned IP address for a PDP context.
    ///
    /// Contexts share the single provisioned IP: the host topology allocates a
    /// /30 subnet per device where CID offsets would hit broadcast or
    /// neighbor subnets. Supporting distinct IPs per APN requires
    /// multi-config host provisioning (TODO: b/562163940).
    pub fn get_ip_address(&self, pdp_type: &PdpType) -> Option<IpAddr> {
        self.find_network_config(pdp_type).map(|cfg| cfg.ip_address)
    }

    pub fn handle_show_pdp_address(&self, cid: u8) -> DataResult {
        if let Some(context) = self.pdp_contexts.get(&cid) {
            if context.active {
                let requested_type =
                    if context.pdp_type == PdpType::Ipv6 { PdpType::Ipv6 } else { PdpType::Ip };
                let ip_address = self
                    .get_ip_address(&requested_type)
                    .ok_or_else(|| ExecutionResult::cme_error(CmeError::NoNetworkService))?;
                Ok(Some(DataResponse::PdpAddress { cid, ip_address }))
            } else {
                Err(ExecutionResult::cme_error(CmeError::InvalidIndex))
            }
        } else {
            Err(ExecutionResult::cme_error(CmeError::InvalidIndex))
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

                let cfg = self
                    .find_network_config(&requested_type)
                    .ok_or_else(|| ExecutionResult::cme_error(CmeError::NoNetworkService))?;
                Ok(Some(DataResponse::DynamicParam {
                    cid,
                    apn,
                    ip_address: cfg.ip_address,
                    prefix: cfg.prefixlen,
                    gateway: cfg.gateway,
                    dns: cfg.dns,
                }))
            } else {
                Err(ExecutionResult::cme_error(CmeError::InvalidIndex))
            }
        } else {
            Err(ExecutionResult::cme_error(CmeError::InvalidIndex))
        }
    }

    pub fn handle_gprs_dial(&mut self, number: &str) -> DataResult {
        if let Some(context) =
            parse_cid_from_gprs_dial(number).and_then(|cid| self.pdp_contexts.get_mut(&cid))
        {
            context.active = true;
            return Ok(Some(DataResponse::Connect));
        }
        Err(ExecutionResult::error())
    }

    pub fn execute<'a>(&mut self, command: &DataCommand<'a>) -> ExecutionResult {
        let result: DataResult = match command {
            DataCommand::DefinePdpContext(cid, pdp_type, apn, ..) => {
                self.handle_define_pdp_context(*cid, *pdp_type, *apn)
            }
            DataCommand::QueryPdpContext => self.handle_query_pdp_context(),
            DataCommand::QueryQualityOfServiceMinimum => self.handle_query_qos(QosType::Minimum),
            DataCommand::SetQualityOfServiceMinimum(cid, qos) => {
                self.handle_set_qos(QosType::Minimum, *cid, *qos)
            }
            DataCommand::SetQualityOfServiceRequested(cid, qos) => {
                self.handle_set_qos(QosType::Requested, *cid, *qos)
            }
            DataCommand::QueryQualityOfServiceRequested => {
                self.handle_query_qos(QosType::Requested)
            }
            DataCommand::SetQualityOfServiceMinimumGprs(cid, qos) => {
                self.handle_set_qos(QosType::MinimumGprs, *cid, *qos)
            }
            DataCommand::QueryQualityOfServiceMinimumGprs => {
                self.handle_query_qos(QosType::MinimumGprs)
            }
            DataCommand::SetQualityOfServiceRequestedGprs(cid, qos) => {
                self.handle_set_qos(QosType::RequestedGprs, *cid, *qos)
            }
            DataCommand::QueryQualityOfServiceRequestedGprs => {
                self.handle_query_qos(QosType::RequestedGprs)
            }
            DataCommand::SetPdpContextActivate(args) => {
                self.handle_set_pdp_context_activate(args.cid, args.state)
            }
            DataCommand::QueryPdpContextActivate => self.handle_query_pdp_context_activate(),
            DataCommand::SetPsAttach(state) => self.handle_set_ps_attach(*state),
            DataCommand::QueryPsAttach => self.handle_query_ps_attach(),
            DataCommand::SetPdpContextModify(cid) => self.handle_set_pdp_context_modify(*cid),
            DataCommand::EnterDataState(l2p, cid) => self.handle_enter_data_state(*l2p, *cid),
            DataCommand::SetPacketEventReporting(_, _) => self.handle_set_packet_event_reporting(),
            DataCommand::ShowPdpAddress(cid) => self.handle_show_pdp_address(*cid),
            DataCommand::ReadDynamicParam(cid) => self.handle_read_dynamic_param(*cid),
        };
        result.into()
    }
}

fn parse_cid_from_gprs_dial(number: &str) -> Option<u8> {
    let trimmed = number.strip_suffix('#')?;
    let parts: Vec<&str> = trimmed.split('*').collect();

    // Expecting a format like *99, *99*<cid>, or *99***<cid> (at most 5 segments)
    if parts.len() < 2 || parts.len() > 5 || !parts[0].is_empty() || parts[1] != "99" {
        return None;
    }

    // Case 1: *99# (parts are ["", "99"])
    if parts.len() == 2 {
        return Some(1);
    }

    // Case 2: *99*<cid># or *99***<cid># (parts.len() > 2)
    let last_part = parts.last()?;
    if last_part.is_empty() {
        return None;
    }
    last_part.parse::<u8>().ok()
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use super::*;
    use crate::{parser::QuotedString, types::Response};

    #[test]
    fn test_data_service_dial_direct() {
        let mut service = DataService::default();
        let res = service.handle_define_pdp_context(1, PdpType::Ip, QuotedString("test"));
        assert!(res.is_ok());

        // Dial
        let res: ExecutionResult = service.handle_gprs_dial("*99***1#").into();
        if let ExecutionResult::Success(handled) = res {
            assert_eq!(handled.responses, vec![Response::Data(DataResponse::Connect)]);
        } else {
            panic!("Expected Success");
        }
    }

    #[test]
    fn test_data_service_dial_malformed() {
        let mut service = DataService::default();
        let res = service.handle_define_pdp_context(1, PdpType::Ip, QuotedString("test"));
        assert!(res.is_ok());

        // Dial malformed alphanumeric CID
        let res: ExecutionResult = service.handle_gprs_dial("*99*abc#").into();
        assert!(matches!(res, ExecutionResult::Error { .. }));

        // Dial empty trailing CID
        let res: ExecutionResult = service.handle_gprs_dial("*99*#").into();
        assert!(matches!(res, ExecutionResult::Error { .. }));
    }

    #[test]
    fn test_pdp_context_auto_activation() {
        let mut service = DataService::default();
        let _ = service.handle_define_pdp_context(1, PdpType::Ip, QuotedString(""));
        assert!(service.pdp_contexts.get(&1).unwrap().active);
    }

    #[test]
    fn test_pdp_context_no_auto_activation_when_detached() {
        let mut service = DataService::default();
        let _ = service.handle_set_ps_attach(false);
        let _ = service.handle_define_pdp_context(1, PdpType::Ip, QuotedString(""));
        assert!(!service.pdp_contexts.get(&1).unwrap().active);
    }

    #[test]
    fn test_network_configs() {
        let network_config = CellNetworkConfig {
            ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 97, 2)),
            prefixlen: 30,
            gateway: IpAddr::V4(Ipv4Addr::new(192, 168, 97, 1)),
            dns: IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
        };

        let service = DataService::new(vec![network_config.clone()]);

        assert_eq!(
            service.get_ip_address(&PdpType::Ip),
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 97, 2)))
        );

        assert_eq!(service.get_ip_address(&PdpType::Ipv6), None);
        assert_eq!(
            service.get_ip_address(&PdpType::Ipv4v6),
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 97, 2)))
        );
        assert_eq!(service.get_ip_address(&PdpType::Ppp), None);

        let v6_config = CellNetworkConfig {
            ip_address: IpAddr::V6(Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 0x15)),
            prefixlen: 64,
            gateway: IpAddr::V6(Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 2)),
            dns: IpAddr::V6(Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 3)),
        };
        let dual_stack_service = DataService::new(vec![network_config.clone(), v6_config.clone()]);
        assert_eq!(
            dual_stack_service.get_ip_address(&PdpType::Ip),
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 97, 2)))
        );
        assert_eq!(
            dual_stack_service.get_ip_address(&PdpType::Ipv6),
            Some(IpAddr::V6(Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 0x15)))
        );

        let mut unconfigured_service = DataService::default();
        assert_eq!(unconfigured_service.get_ip_address(&PdpType::Ip), None);
        assert_eq!(unconfigured_service.get_ip_address(&PdpType::Ipv6), None);
        assert_eq!(unconfigured_service.get_ip_address(&PdpType::Ipv4v6), None);
        let _ =
            unconfigured_service.handle_define_pdp_context(1, PdpType::Ip, QuotedString("test"));
        assert!(unconfigured_service.handle_show_pdp_address(1).is_err());
        assert!(unconfigured_service.handle_read_dynamic_param(1).is_err());

        unconfigured_service.update_network_configs(vec![network_config.clone()]);
        assert_eq!(
            unconfigured_service.get_ip_address(&PdpType::Ip),
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 97, 2)))
        );
        assert_eq!(unconfigured_service.network_configs(), &[network_config]);
        assert!(unconfigured_service.handle_show_pdp_address(1).is_ok());
        assert!(unconfigured_service.handle_read_dynamic_param(1).is_ok());
    }

    #[test]
    fn test_dynamic_param_prefix_passthrough() {
        let mut service_v6 = DataService::new(vec![CellNetworkConfig {
            ip_address: IpAddr::V6(Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 0x15)),
            prefixlen: 64,
            gateway: IpAddr::V6(Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 2)),
            dns: IpAddr::V6(Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 3)),
        }]);
        service_v6.pdp_contexts.insert(
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

        let res = service_v6.handle_read_dynamic_param(1).unwrap();
        match res {
            Some(DataResponse::DynamicParam { prefix, .. }) => {
                assert_eq!(prefix, 64);
            }
            _ => panic!("Expected DynamicParam response"),
        }

        // Prefix is passed through verbatim without validation.
        let mut service_zero = DataService::new(vec![CellNetworkConfig {
            ip_address: IpAddr::V6(Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 0x15)),
            prefixlen: 0,
            gateway: IpAddr::V6(Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 2)),
            dns: IpAddr::V6(Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 3)),
        }]);
        service_zero.pdp_contexts.insert(
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
        match service_zero.handle_read_dynamic_param(1).unwrap() {
            Some(DataResponse::DynamicParam { prefix, .. }) => assert_eq!(prefix, 0),
            _ => panic!("Expected DynamicParam response"),
        }
    }
}
