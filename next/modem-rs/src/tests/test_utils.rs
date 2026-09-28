// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{
    CardState, RadioAdmission, SmsStore,
    capabilities::AdmissionDenial,
    types::{CmeError, CmsError, PhoneNumber, SimSmsMessage},
};

/// Mock implementation of [`CardState`] for unit tests.
#[derive(Debug, Clone)]
pub(crate) struct MockCardState {
    pub is_present: bool,
    pub is_ready: bool,
    pub is_fdn_allowed: bool,
    pub gating_error: Option<CmeError>,
}

impl Default for MockCardState {
    fn default() -> Self {
        Self { is_present: true, is_ready: true, is_fdn_allowed: true, gating_error: None }
    }
}

impl CardState for MockCardState {
    fn is_present(&self) -> bool {
        self.is_present
    }
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn validate_call(&self, _number: &PhoneNumber, is_emergency: bool) -> Result<(), CmeError> {
        if is_emergency {
            return Ok(());
        }
        if let Some(err) = self.gating_error {
            return Err(err);
        }
        if !self.is_fdn_allowed {
            return Err(CmeError::FixedDialNumberOnlyAllowed);
        }
        Ok(())
    }
}

impl MockCardState {
    fn check_sms_access(&self) -> Result<(), CmsError> {
        if !self.is_present() {
            Err(CmsError::SimNotInserted)
        } else if !self.is_ready() {
            Err(CmsError::SimPinRequired)
        } else {
            Ok(())
        }
    }
}

impl SmsStore for MockCardState {
    fn store_sms(&mut self, _message: SimSmsMessage) -> Result<u8, CmsError> {
        self.check_sms_access().map(|_| 1)
    }
    fn read_sms(&mut self, _index: u8) -> Result<SimSmsMessage, CmsError> {
        self.check_sms_access()?;
        Err(CmsError::InvalidMemoryIndex)
    }
    fn delete_sms(&mut self, _index: u8) -> Result<(), CmsError> {
        self.check_sms_access()?;
        Err(CmsError::InvalidMemoryIndex)
    }
}

/// Mock implementation of [`RadioAdmission`] for unit tests.
#[derive(Debug, Clone)]
pub(crate) struct MockRadioAdmission {
    pub admission: Result<(), AdmissionDenial>,
}

impl Default for MockRadioAdmission {
    fn default() -> Self {
        Self { admission: Ok(()) }
    }
}

impl RadioAdmission for MockRadioAdmission {
    fn can_originate_voice_call(&self, _is_emergency: bool) -> Result<(), AdmissionDenial> {
        self.admission
    }
    fn can_originate_data(&self) -> Result<(), AdmissionDenial> {
        self.admission
    }
    fn can_terminate_voice_call(&self) -> bool {
        self.admission.is_ok()
    }
    fn can_originate_sms(&self) -> Result<(), AdmissionDenial> {
        self.admission
    }
    fn can_terminate_sms(&self) -> bool {
        self.admission.is_ok()
    }
}
