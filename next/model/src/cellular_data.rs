// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CellularData {
    #[serde(flatten)]
    pub radio: crate::chip::Radio,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CellularDataUpdate {
    #[serde(flatten)]
    pub radio: crate::chip::RadioUpdate,
}

impl CellularDataUpdate {
    pub fn apply(&self, cellular_data: &mut CellularData) {
        self.radio.apply(&mut cellular_data.radio);
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellularDataCreate {
    // Future Cellular Data specific properties.
}

impl From<CellularDataCreate> for CellularData {
    fn from(_create: CellularDataCreate) -> Self {
        CellularData { radio: crate::chip::Radio::default() }
    }
}
