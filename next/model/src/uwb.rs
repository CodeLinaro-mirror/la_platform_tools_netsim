// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Uwb {
    #[serde(flatten)]
    pub radio: crate::chip::Radio,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UwbUpdate {
    #[serde(flatten)]
    pub radio: crate::chip::RadioUpdate,
}

impl UwbUpdate {
    pub fn apply(&self, uwb: &mut Uwb) {
        self.radio.apply(&mut uwb.radio);
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct UwbCreate {
    // Future UWB specific properties.
}
