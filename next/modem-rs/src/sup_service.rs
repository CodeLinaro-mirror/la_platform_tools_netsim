// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use modem_rs_derive::CommandParser;

use crate::{
    parser::{QuotedString, parse_raw_data},
    sim_service::SimService,
    types::{
        CallForwardingMode, CallForwardingReason, CallWaitingMode, CallWaitingPresentation,
        CallWaitingStatus, ClipActivation, ClipProvisionStatus, ClirMode, ClirStatus, CmeError,
        ExecutionResult, Facility, FacilityLockMode, Parsable, UssdMode, UssdStatus,
    },
};

/// Supplementary service AT commands.
#[derive(Debug, PartialEq, Clone, Copy, CommandParser)]
pub enum SupCommand<'a> {
    #[command(tag = "AT+CLCK=")]
    SetFacilityLock(Facility, FacilityLockMode, Option<QuotedString<'a>>, Option<u8>),
    #[command(tag = "AT+CCFC=")]
    CallForwarding {
        reason: CallForwardingReason,
        mode: CallForwardingMode,
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
    SetClirGoldfish(ClirMode),
    #[command(tag = "AT+CLIR=")]
    SetClir(ClirMode),
    #[command(tag = "AT+CLIP=")]
    SetClip(ClipActivation),
    #[command(tag = "AT+CLIP?")]
    QueryClip,
    #[command(tag = "AT+COLP=")]
    SetColp(u8),
    #[command(tag = "AT+CCWA=")]
    SetCallWaiting(CallWaitingPresentation, Option<CallWaitingMode>, Option<u8>),
    #[command(tag = "AT+CSSN=")]
    SuppServiceNotification(u8, u8),
    #[command(tag = "AT+CUSD=")]
    SetUssd { mode: UssdMode, message: Option<QuotedString<'a>>, dcs: Option<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupResponse {
    FacilityLockStatus(u8),
    Clir { n: ClirMode, m: ClirStatus },
    Clip { activation: ClipActivation, provision: ClipProvisionStatus },
    CallWaiting { status: CallWaitingStatus, class: u8 },
    Ussd { status: UssdStatus, message: String, dcs: u8 },
}

impl std::fmt::Display for SupResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupResponse::FacilityLockStatus(status) => write!(f, "+CLCK: {status}\r\n"),
            SupResponse::Clir { n, m } => write!(f, "+CLIR: {n},{m}\r\n"),
            SupResponse::Clip { activation, provision } => {
                write!(f, "+CLIP: {activation},{provision}\r\n")
            }
            SupResponse::CallWaiting { status, class } => {
                write!(f, "+CCWA: {status},{class}\r\n")
            }
            SupResponse::Ussd { status, message, dcs } => {
                write!(f, "+CUSD: {status},\"{message}\",{dcs}\r\n")
            }
        }
    }
}

type SupResult = Result<Option<SupResponse>, ExecutionResult>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallForwardingInfo {
    pub mode: CallForwardingMode,
    pub number: String,
    pub type_: u8,
}

#[derive(Debug, Default)]
pub struct SupService {
    call_forwarding_info: Option<CallForwardingInfo>,
    clip_enabled: ClipActivation,
}

impl SupService {
    pub fn clip_enabled(&self) -> bool {
        self.clip_enabled == ClipActivation::Enable
    }

    // --- Pure command handlers ---

    fn handle_set_facility_lock(&self, mode: FacilityLockMode) -> SupResult {
        if mode == FacilityLockMode::QueryStatus {
            Ok(Some(SupResponse::FacilityLockStatus(0)))
        } else {
            Ok(None)
        }
    }

    fn handle_call_forwarding(
        &mut self,
        mode: CallForwardingMode,
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

        self.call_forwarding_info = Some(info);
        Ok(None)
    }

    fn handle_query_clir(&self) -> SupResult {
        Ok(Some(SupResponse::Clir { n: ClirMode::SubscriptionDefault, m: ClirStatus::NotActive }))
    }

    fn handle_set_clir(&self, _clir: ClirMode) -> SupResult {
        Ok(None)
    }

    fn handle_set_clip(&mut self, enabled: ClipActivation) -> SupResult {
        self.clip_enabled = enabled;
        Ok(None)
    }

    fn handle_query_clip(&self) -> SupResult {
        Ok(Some(SupResponse::Clip {
            activation: self.clip_enabled,
            provision: ClipProvisionStatus::Provisioned,
        }))
    }

    fn handle_set_call_waiting(
        &self,
        _n: CallWaitingPresentation,
        mode: Option<CallWaitingMode>,
        class: Option<u8>,
    ) -> SupResult {
        if let Some(CallWaitingMode::Query) = mode {
            let classx = class.unwrap_or(7);
            Ok(Some(SupResponse::CallWaiting {
                status: CallWaitingStatus::NotActive,
                class: classx,
            }))
        } else {
            Ok(None)
        }
    }

    fn handle_set_ussd(
        &self,
        mode: UssdMode,
        message: Option<QuotedString>,
        _dcs: Option<u8>,
    ) -> SupResult {
        if mode == UssdMode::EnableUrc && message.is_some() {
            Ok(Some(SupResponse::Ussd {
                status: UssdStatus::NoActionRequired,
                message: "OK".to_string(),
                dcs: 15,
            }))
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
            SupCommand::SetFacilityLock(facility, mode, passwd, _) => match facility {
                Facility::SimPin => {
                    return sim_service.handle_set_facility_lock(*mode, *passwd).into();
                }
                Facility::Other => self.handle_set_facility_lock(*mode),
            },
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
