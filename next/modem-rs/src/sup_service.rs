// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use modem_rs_derive::CommandParser;

use crate::{
    parser::QuotedString,
    sim_service::SimService,
    types::{
        CallForwardUtilityArgs, CallForwardingMode, CallForwardingReason, CallWaitingMode,
        CallWaitingStatus, ClipProvisionStatus, ClirMode, ClirStatus, CmeError, ExecutionResult,
        Facility, FacilityLockMode, Parsable, PhoneNumber, ServiceClass, TypeOfAddress, UssdMode,
        UssdStatus,
    },
};

/// Supplementary service AT commands.
#[derive(Debug, PartialEq, Clone, CommandParser)]
pub enum SupCommand<'a> {
    #[command(tag = "AT+CLCK=")]
    SetFacilityLock(Facility, FacilityLockMode, Option<QuotedString<'a>>, Option<ServiceClass>),
    #[command(tag = "AT+CCFC=")]
    CallForwarding {
        reason: CallForwardingReason,
        mode: CallForwardingMode,
        number: Option<PhoneNumber>,
        toa: Option<TypeOfAddress>,
        class: Option<ServiceClass>,
        subaddr: Option<QuotedString<'a>>,
        satype: Option<u8>,
        time: Option<u8>,
    },
    #[command(tag = "AT+CCFCU=")]
    CallForwardUtility(CallForwardUtilityArgs<'a>),
    #[command(tag = "AT+CLIR?")]
    QueryClir,
    /// Non-standard Goldfish CLIR syntax
    #[command(tag = "AT+CLIR: ")]
    SetClirGoldfish(ClirMode),
    #[command(tag = "AT+CLIR=")]
    SetClir(ClirMode),
    #[command(tag = "AT+CLIP=")]
    SetClip(bool),
    #[command(tag = "AT+CLIP?")]
    QueryClip,
    #[command(tag = "AT+COLP=")]
    SetColp(bool),
    #[command(tag = "AT+CCWA=")]
    SetCallWaiting(bool, Option<CallWaitingMode>, Option<ServiceClass>),
    #[command(tag = "AT+CSSN=")]
    SuppServiceNotification(bool, Option<bool>),
    #[command(tag = "AT+CUSD=")]
    SetUssd { mode: UssdMode, message: Option<QuotedString<'a>>, dcs: Option<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupResponse {
    FacilityLockStatus(u8),
    Clir { n: ClirMode, m: ClirStatus },
    Clip { activation: bool, provision: ClipProvisionStatus },
    CallWaiting(Vec<(CallWaitingStatus, u8)>),
    Ussd { status: UssdStatus, message: String, dcs: u8 },
}

impl std::fmt::Display for SupResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupResponse::FacilityLockStatus(status) => write!(f, "+CLCK: {status}\r\n"),
            SupResponse::Clir { n, m } => write!(f, "+CLIR: {n},{m}\r\n"),
            SupResponse::Clip { activation, provision } => {
                write!(f, "+CLIP: {},{provision}\r\n", *activation as u8)
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
    pub number: Option<PhoneNumber>,
    pub toa: TypeOfAddress,
}

#[derive(Debug)]
pub struct SupService {
    call_forwarding_info: Option<CallForwardingInfo>,
    clip_enabled: bool,
    clir_mode: ClirMode,
    ccwa_presentation: bool,
    ccwa_status: BTreeMap<ServiceClass, CallWaitingStatus>,
}

impl Default for SupService {
    fn default() -> Self {
        let ccwa_status = BTreeMap::from([
            (ServiceClass::VOICE, CallWaitingStatus::NotActive),
            (ServiceClass::DATA, CallWaitingStatus::NotActive),
            (ServiceClass::FAX, CallWaitingStatus::NotActive),
        ]);
        Self {
            call_forwarding_info: None,
            clip_enabled: false,
            clir_mode: ClirMode::default(),
            ccwa_presentation: false,
            ccwa_status,
        }
    }
}

impl SupService {
    pub fn clip_enabled(&self) -> bool {
        self.clip_enabled
    }

    pub fn clir_mode(&self) -> ClirMode {
        self.clir_mode
    }

    // --- Pure command handlers ---

    fn handle_call_forwarding(
        &mut self,
        mode: CallForwardingMode,
        number: Option<PhoneNumber>,
        toa: Option<TypeOfAddress>,
    ) -> SupResult {
        let toa = toa
            .unwrap_or_else(|| number.as_ref().map(|n| n.toa()).unwrap_or(TypeOfAddress::Unknown));
        let info = CallForwardingInfo { mode, number, toa };

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
        presentation: bool,
        mode: Option<CallWaitingMode>,
        class: Option<ServiceClass>,
    ) -> SupResult {
        self.ccwa_presentation = presentation;
        let class = class.unwrap_or_default();
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
                    if class.contains(bc) && status == CallWaitingStatus::Active {
                        active_classes.push(bc.as_u8());
                    }
                }
                if active_classes.is_empty() {
                    // None are active, return single line indicating disabled for the queried class
                    Ok(Some(SupResponse::CallWaiting(vec![(
                        CallWaitingStatus::NotActive,
                        class.as_u8(),
                    )])))
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

    fn set_ccwa_status(&mut self, class: ServiceClass, status: CallWaitingStatus) {
        for (&bc, val) in &mut self.ccwa_status {
            if class.contains(bc) {
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
            SupCommand::SetFacilityLock(facility, mode, passwd, _) => {
                return sim_service.handle_set_facility_lock(*facility, *mode, *passwd).into();
            }
            SupCommand::CallForwarding { reason: _, mode, number, toa, .. } => {
                self.handle_call_forwarding(*mode, number.clone(), *toa)
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
            SupCommand::SetCallWaiting(presentation, mode, class) => {
                self.handle_set_call_waiting(*presentation, *mode, *class)
            }
            SupCommand::SetUssd { mode, message, dcs } => {
                self.handle_set_ussd(*mode, *message, *dcs)
            }
        };

        sup_result.into()
    }
}
