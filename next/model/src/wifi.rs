// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Wifi {
    #[serde(flatten)]
    pub radio: crate::chip::Radio,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WifiUpdate {
    #[serde(flatten)]
    pub radio: crate::chip::RadioUpdate,
}

impl WifiUpdate {
    pub fn apply(&self, wifi: &mut Wifi) {
        self.radio.apply(&mut wifi.radio);
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct WifiCreate {
    // Future Wi-Fi specific properties.
}
