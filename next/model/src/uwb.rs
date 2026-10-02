// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UwbMode {
    #[default]
    Device,
    Anchor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Uwb {
    #[serde(flatten)]
    pub radio: crate::chip::Radio,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub mode: UwbMode,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum UwbCreate {
    #[default]
    Device,
    Anchor {
        #[serde(default)]
        address: Option<String>,
    },
}

impl From<UwbCreate> for Uwb {
    fn from(create: UwbCreate) -> Self {
        match create {
            UwbCreate::Device => {
                Self { radio: Default::default(), address: String::new(), mode: UwbMode::Device }
            }
            UwbCreate::Anchor { address } => Self {
                radio: Default::default(),
                address: address.unwrap_or_default(),
                mode: UwbMode::Anchor,
            },
        }
    }
}
