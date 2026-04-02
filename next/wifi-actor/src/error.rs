// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::chip_error::ChipError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WifiError {
    #[error("Hostapd error: {0}")]
    Hostapd(Box<dyn std::error::Error + Send + Sync>),
    #[error("Network error: {0}")]
    Network(Box<dyn std::error::Error + Send + Sync>),
    #[error("Client error: {0}")]
    Client(Box<dyn std::error::Error + Send + Sync>),
    #[error("Frame error: {0}")]
    Frame(Box<dyn std::error::Error + Send + Sync>),
    #[error("Transmission error: {0}")]
    Transmission(Box<dyn std::error::Error + Send + Sync>),
    #[error("Chip error: {0}")]
    Chip(#[from] ChipError),
    #[error("Other error: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
    #[error("Internal error: {0}")]
    Internal(Box<dyn std::error::Error + Send + Sync>),
}
