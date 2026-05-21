// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug)]
pub enum EthernetError {
    InvalidChipKind,
    Internal(String),
}

impl std::fmt::Display for EthernetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EthernetError::InvalidChipKind => write!(f, "Invalid ChipKind for EthernetActor"),
            EthernetError::Internal(s) => write!(f, "Internal error: {s}"),
        }
    }
}

impl std::error::Error for EthernetError {}
