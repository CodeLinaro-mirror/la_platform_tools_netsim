// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Ethernet {
    #[serde(flatten)]
    pub radio: crate::chip::Radio,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EthernetUpdate {
    #[serde(flatten)]
    pub radio: crate::chip::RadioUpdate,
}

impl EthernetUpdate {
    pub fn apply(&self, ethernet: &mut Ethernet) {
        self.radio.apply(&mut ethernet.radio);
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct EthernetCreate {
    // Future Ethernet specific properties.
}

impl From<EthernetCreate> for Ethernet {
    fn from(_create: EthernetCreate) -> Self {
        Ethernet { radio: crate::chip::Radio::default() }
    }
}
