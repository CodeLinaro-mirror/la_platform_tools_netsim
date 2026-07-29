// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Nfc {
    #[serde(flatten)]
    pub radio: crate::chip::Radio,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NfcUpdate {
    #[serde(flatten)]
    pub radio: crate::chip::RadioUpdate,
}

impl NfcUpdate {
    pub fn apply(&self, nfc: &mut Nfc) {
        self.radio.apply(&mut nfc.radio);
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct NfcCreate {
    // Future NFC specific properties.
}

impl From<NfcCreate> for Nfc {
    fn from(_create: NfcCreate) -> Self {
        Nfc { radio: crate::chip::Radio::default() }
    }
}
