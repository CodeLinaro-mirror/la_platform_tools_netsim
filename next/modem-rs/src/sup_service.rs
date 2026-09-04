// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

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

const SERVICE_CLASS_VOICE: u8 = 1;
const SERVICE_CLASS_DATA: u8 = 2;
const SERVICE_CLASS_FAX: u8 = 4;
const SERVICE_CLASS_VOICE_DATA_FAX: u8 = 7;

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
    CallWaiting(Vec<(CallWaitingStatus, u8)>),
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
            SupResponse::CallWaiting(infos) => {
                for (status, class) in infos {
                    write!(f, "+CCWA: {status},{class}\r\n")?;
                }
                Ok(())
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

#[derive(Debug)]
pub struct SupService {
    call_forwarding_info: Option<CallForwardingInfo>,
    clip_enabled: ClipActivation,
    clir_mode: ClirMode,
    ccwa_presentation: CallWaitingPresentation,
    ccwa_status: BTreeMap<u8, CallWaitingStatus>,
}

impl Default for SupService {
    fn default() -> Self {
        let ccwa_status = BTreeMap::from([
            (SERVICE_CLASS_VOICE, CallWaitingStatus::NotActive),
            (SERVICE_CLASS_DATA, CallWaitingStatus::NotActive),
            (SERVICE_CLASS_FAX, CallWaitingStatus::NotActive),
        ]);
        Self {
            call_forwarding_info: None,
            clip_enabled: ClipActivation::default(),
            clir_mode: ClirMode::default(),
            ccwa_presentation: CallWaitingPresentation::Disable,
            ccwa_status,
        }
    }
}

impl SupService {
    pub fn clip_enabled(&self) -> bool {
        self.clip_enabled == ClipActivation::Enable
    }

    pub fn clir_mode(&self) -> ClirMode {
        self.clir_mode
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
        Ok(Some(SupResponse::Clir { n: self.clir_mode, m: ClirStatus::Active }))
    }

    fn handle_query_clip(&self) -> SupResult {
        Ok(Some(SupResponse::Clip {
            activation: self.clip_enabled,
            provision: ClipProvisionStatus::Provisioned,
        }))
    }

    fn handle_set_call_waiting(
        &mut self,
        n: CallWaitingPresentation,
        mode: Option<CallWaitingMode>,
        class: Option<u8>,
    ) -> SupResult {
        self.ccwa_presentation = n;
        let class = class.unwrap_or(SERVICE_CLASS_VOICE_DATA_FAX);
        match mode {
            Some(CallWaitingMode::Disable) => {
                self.set_ccwa_status(class, CallWaitingStatus::NotActive);
                Ok(None)
            }
            Some(CallWaitingMode::Enable) => {
                self.set_ccwa_status(class, CallWaitingStatus::Active);
                Ok(None)
            }
            Some(CallWaitingMode::Query) => {
                // Find all active basic classes that are subset of the queried class.
                let mut active_classes = Vec::new();
                for (&bc, &status) in &self.ccwa_status {
                    if (class & bc) == bc && status == CallWaitingStatus::Active {
                        active_classes.push(bc);
                    }
                }
                if active_classes.is_empty() {
                    // None are active, return single line indicating disabled for the queried class
                    Ok(Some(SupResponse::CallWaiting(vec![(CallWaitingStatus::NotActive, class)])))
                } else {
                    // Return active classes
                    let infos = active_classes
                        .into_iter()
                        .map(|bc| (CallWaitingStatus::Active, bc))
                        .collect();
                    Ok(Some(SupResponse::CallWaiting(infos)))
                }
            }
            None => Ok(None),
        }
    }

    fn set_ccwa_status(&mut self, class: u8, status: CallWaitingStatus) {
        for (&bc, val) in &mut self.ccwa_status {
            if (class & bc) == bc {
                *val = status;
            }
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
                Facility::FixedDial => {
                    return sim_service.handle_set_fdn_lock(*mode, *passwd).into();
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
                self.clir_mode = *clir;
                Ok(None)
            }
            SupCommand::SetClip(enabled) => {
                self.clip_enabled = *enabled;
                Ok(None)
            }
            SupCommand::QueryClip => self.handle_query_clip(),
            SupCommand::SetColp(_) | SupCommand::SuppServiceNotification(_, _) => Ok(None),
            SupCommand::SetCallWaiting(n, mode, class) => {
                self.handle_set_call_waiting(*n, *mode, *class)
            }
            SupCommand::SetUssd { mode, message, dcs } => {
                self.handle_set_ussd(*mode, *message, *dcs)
            }
        };

        sup_result.into()
    }
}
