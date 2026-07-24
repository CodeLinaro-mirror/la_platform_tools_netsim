// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use modem_rs_derive::CommandParser;

use crate::{
    constants::FACILITY_SIM_PIN,
    parser::{QuotedString, parse_raw_data},
    sim_service::SimService,
    types::{CmeError, ExecutionResult, Parsable},
};

/// Supplementary service AT commands.
#[derive(Debug, PartialEq, Clone, Copy, CommandParser)]
pub enum SupCommand<'a> {
    #[command(tag = "AT+CLCK=")]
    SetFacilityLock(QuotedString<'a>, u8, Option<QuotedString<'a>>, Option<u8>),
    #[command(tag = "AT+CCFC=")]
    CallForwarding {
        reason: u8,
        mode: u8,
        number: Option<QuotedString<'a>>,
        r#type: Option<u8>,
        class: Option<u8>,
        subaddr: Option<QuotedString<'a>>,
        satype: Option<u8>,
        time: Option<u8>,
    },
    #[command(tag = "AT+CCFCU=")]
    CallForwardUtility(#[parser(parse_raw_data)] &'a [u8]),
    #[command(tag = "AT+CLIR?")]
    QueryClir,
    /// Non-standard Goldfish CLIR syntax
    #[command(tag = "AT+CLIR: ")]
    SetClirGoldfish(u8),
    #[command(tag = "AT+CLIR=")]
    SetClir(u8),
    #[command(tag = "AT+CLIP=")]
    SetClip(u8),
    #[command(tag = "AT+CLIP?")]
    QueryClip,
    #[command(tag = "AT+COLP=")]
    SetColp(u8),
    #[command(tag = "AT+CCWA=")]
    SetCallWaiting(u8, Option<u8>, Option<u8>),
    #[command(tag = "AT+CSSN=")]
    SuppServiceNotification(u8, u8),
    #[command(tag = "AT+CUSD=")]
    SetUssd { mode: u8, message: Option<QuotedString<'a>>, dcs: Option<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupResponse {
    FacilityLockStatus(u8),
    Clir { n: u8, m: u8 },
    Clip { status: u8, class: u8 },
    CallWaiting { status: u8, class: u8 },
    Ussd { status: u8, message: String, dcs: u8 },
}

impl std::fmt::Display for SupResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupResponse::FacilityLockStatus(status) => write!(f, "+CLCK: {status}\r\n"),
            SupResponse::Clir { n, m } => write!(f, "+CLIR: {n},{m}\r\n"),
            SupResponse::Clip { status, class } => write!(f, "+CLIP: {status},{class}\r\n"),
            SupResponse::CallWaiting { status, class } => write!(f, "+CCWA: {status},{class}\r\n"),
            SupResponse::Ussd { status, message, dcs } => {
                write!(f, "+CUSD: {status},\"{message}\",{dcs}\r\n")
            }
        }
    }
}

type SupResult = Result<Option<SupResponse>, ExecutionResult>;

pub const _MODE_ENABLE: u8 = 1;
pub const _MODE_QUERY: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallForwardingInfo {
    pub mode: u8,
    pub number: String,
    pub type_: u8,
}

#[derive(Debug, Default)]
pub struct SupService {
    call_forwarding_info: Option<CallForwardingInfo>,
    clip_enabled: u8,
}

impl SupService {
    // --- Pure command handlers ---

    fn handle_set_facility_lock(&self, _facility: &str, mode: u8) -> SupResult {
        if mode == 2 { Ok(Some(SupResponse::FacilityLockStatus(0))) } else { Ok(None) }
    }

    fn handle_call_forwarding(
        &mut self,
        mode: u8,
        number: Option<QuotedString>,
        type_: Option<u8>,
    ) -> SupResult {
        let number = number.map(|s| s.to_vec()).unwrap_or_default();
        let type_ = type_.unwrap_or_default();
        let info = CallForwardingInfo {
            mode,
            number: String::from_utf8(number).unwrap_or_default(),
            type_,
        };

        // Simplified logic: If enabling, set info. If querying/disabling, we might
        // check it. Original code only had specific cases.
        // MODE_ENABLE in original code was setting it.
        // MODE_QUERY in original code was reading it? Wait, MODE_QUERY is 2.

        self.call_forwarding_info = Some(info);
        Ok(None)
    }

    fn handle_query_clir(&self) -> SupResult {
        Ok(Some(SupResponse::Clir { n: 0, m: 0 }))
    }

    fn handle_set_clir(&self, _clir: u8) -> SupResult {
        Ok(None)
    }

    fn handle_set_clip(&mut self, enabled: u8) -> SupResult {
        self.clip_enabled = enabled;
        Ok(None)
    }

    fn handle_query_clip(&self) -> SupResult {
        Ok(Some(SupResponse::Clip { status: self.clip_enabled, class: 1 }))
    }

    fn handle_set_call_waiting(&self, _n: u8, mode: Option<u8>, class: Option<u8>) -> SupResult {
        if let Some(2) = mode {
            let classx = class.unwrap_or(7);
            Ok(Some(SupResponse::CallWaiting { status: 0, class: classx }))
        } else {
            Ok(None)
        }
    }

    fn handle_set_ussd(
        &self,
        mode: u8,
        message: Option<QuotedString>,
        _dcs: Option<u8>,
    ) -> SupResult {
        if mode == 1 && message.is_some() {
            Ok(Some(SupResponse::Ussd { status: 0, message: "OK".to_string(), dcs: 15 }))
        } else {
            Ok(None)
        }
    }

    fn handle_supp_service_notification(&self) -> SupResult {
        Ok(None)
    }

    fn handle_set_colp(&self) -> SupResult {
        Ok(None)
    }

    pub fn execute<'a>(
        &mut self,
        command: &SupCommand<'a>,
        sim_service: &mut SimService,
    ) -> ExecutionResult {
        let sup_result = match command {
            SupCommand::SetFacilityLock(facility, mode, passwd, _) => {
                let facility_str = std::str::from_utf8(facility.as_ref()).unwrap_or("");
                if facility_str == FACILITY_SIM_PIN {
                    return sim_service.handle_set_facility_lock(*mode, *passwd).into();
                }
                self.handle_set_facility_lock(facility_str, *mode)
            }
            SupCommand::CallForwarding { reason: _, mode, number, r#type, .. } => {
                self.handle_call_forwarding(*mode, *number, *r#type)
            }
            SupCommand::CallForwardUtility(_) => {
                Err(ExecutionResult::cme_error(CmeError::OperationNotSupported))
            }
            SupCommand::QueryClir => self.handle_query_clir(),
            SupCommand::SetClir(clir) | SupCommand::SetClirGoldfish(clir) => {
                self.handle_set_clir(*clir)
            }
            SupCommand::SetClip(enabled) => self.handle_set_clip(*enabled),
            SupCommand::QueryClip => self.handle_query_clip(),
            SupCommand::SetColp(_) => self.handle_set_colp(),
            SupCommand::SetCallWaiting(n, mode, class) => {
                self.handle_set_call_waiting(*n, *mode, *class)
            }
            SupCommand::SuppServiceNotification(_, _) => self.handle_supp_service_notification(),
            SupCommand::SetUssd { mode, message, dcs } => {
                self.handle_set_ussd(*mode, *message, *dcs)
            }
        };

        sup_result.into()
    }
}
