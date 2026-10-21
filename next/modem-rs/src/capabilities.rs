// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::RegistrationStatus;

use crate::types::{CmeError, CmsError, ExecutionResult, PhoneNumber, SimSmsMessage};

pub(crate) trait CardState {
    /// Physically inserted and provisioned. True even for a PIN-locked
    /// card — distinct from `is_ready`.
    fn is_present(&self) -> bool;
    fn is_ready(&self) -> bool;
    fn validate_call(&self, number: &PhoneNumber, is_emergency: bool) -> Result<(), CmeError>;
}

/// SIM-resident SMS record storage (3GPP TS 51.011 EF_SMS).
pub(crate) trait SmsStore: CardState {
    fn store_sms(&mut self, message: SimSmsMessage) -> Result<u8, CmsError>;
    fn read_sms(&mut self, index: u8) -> Result<SimSmsMessage, CmsError>;
    fn delete_sms(&mut self, index: u8) -> Result<(), CmsError>;
}

pub(crate) trait RadioAdmission {
    fn can_originate_voice_call(&self, is_emergency: bool) -> Result<(), AdmissionDenial>;
    fn can_originate_data(&self) -> Result<(), AdmissionDenial>;
    fn can_terminate_voice_call(&self) -> bool;
    fn can_originate_sms(&self) -> Result<(), AdmissionDenial>;
    fn can_terminate_sms(&self) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdmissionDenial {
    NoNetworkService,
    EmergencyCallsOnly,
}

impl AdmissionDenial {
    pub(crate) fn check_registration(reg: RegistrationStatus) -> Result<(), Self> {
        if reg.is_emergency_only() {
            Err(Self::EmergencyCallsOnly)
        } else if !reg.is_registered() {
            Err(Self::NoNetworkService)
        } else {
            Ok(())
        }
    }
}

impl From<AdmissionDenial> for CmeError {
    fn from(denial: AdmissionDenial) -> Self {
        match denial {
            AdmissionDenial::NoNetworkService => CmeError::NoNetworkService,
            AdmissionDenial::EmergencyCallsOnly => CmeError::NetworkNotAllowedEmergencyCallsOnly,
        }
    }
}

impl From<AdmissionDenial> for CmsError {
    fn from(denial: AdmissionDenial) -> Self {
        match denial {
            AdmissionDenial::NoNetworkService | AdmissionDenial::EmergencyCallsOnly => {
                CmsError::NoNetworkService
            }
        }
    }
}

impl From<AdmissionDenial> for ExecutionResult {
    fn from(denial: AdmissionDenial) -> Self {
        CmeError::from(denial).into()
    }
}
