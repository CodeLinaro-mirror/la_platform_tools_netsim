// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{
    constants::FACILITY_SIM_PIN,
    parser::{Command, QuotedString},
    types::{CmeError, ExecutionResult, HandledCommand},
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum SupResponse {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SupError {
    Cme(CmeError),
    Unhandled,
}

impl From<CmeError> for SupError {
    fn from(err: CmeError) -> Self {
        SupError::Cme(err)
    }
}

type SupResult = Result<Option<SupResponse>, SupError>;

impl From<SupResult> for ExecutionResult {
    fn from(res: SupResult) -> Self {
        match res {
            Ok(opt_resp) => {
                let mut handled = HandledCommand::ok();
                if let Some(resp) = opt_resp {
                    let resp_str = resp.to_string();
                    if !resp_str.is_empty() {
                        handled.responses.insert(0, resp_str);
                    }
                }
                ExecutionResult::Success(handled)
            }
            Err(SupError::Cme(err)) => ExecutionResult::CmeError(err),
            Err(SupError::Unhandled) => ExecutionResult::Unhandled,
        }
    }
}

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

    pub fn execute(&mut self, command: &Command) -> ExecutionResult {
        let sup_result = match command {
            Command::SetFacilityLock(facility, mode, _, _) => {
                let facility_str = std::str::from_utf8(facility.as_ref()).unwrap_or("");
                if facility_str != FACILITY_SIM_PIN {
                    self.handle_set_facility_lock(facility_str, *mode)
                } else {
                    Err(SupError::Unhandled)
                }
            }
            Command::CallForwarding { reason: _, mode, number, r#type, .. } => {
                self.handle_call_forwarding(*mode, *number, *r#type)
            }
            Command::CallForwardUtility(_) => Err(SupError::Cme(CmeError::OperationNotSupported)),
            Command::QueryClir => self.handle_query_clir(),
            Command::SetClir(clir) | Command::SetClirGoldfish(clir) => self.handle_set_clir(*clir),
            Command::SetClip(enabled) => self.handle_set_clip(*enabled),
            Command::QueryClip => self.handle_query_clip(),
            Command::SetColp(_) => self.handle_set_colp(),
            Command::SetCallWaiting(n, mode, class) => {
                self.handle_set_call_waiting(*n, *mode, *class)
            }
            Command::SuppServiceNotification(_, _) => self.handle_supp_service_notification(),
            Command::SetUssd { mode, message, dcs } => self.handle_set_ussd(*mode, *message, *dcs),
            _ => Err(SupError::Unhandled),
        };

        sup_result.into()
    }
}
