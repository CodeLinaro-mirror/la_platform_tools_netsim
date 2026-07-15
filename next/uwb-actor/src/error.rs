// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::ChipError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UwbError {
    #[error("{0}")]
    Chip(#[from] ChipError),

    #[error("Pica shutdown unexpectedly")]
    PicaShutdown(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Invalid azimuth: {0} (expected -180.0 to 180.0)")]
    InvalidAzimuth(f32),

    #[error("Invalid elevation: {0} (expected -90.0 to 90.0)")]
    InvalidElevation(f32),

    #[error("Packet stream is missing")]
    PacketStreamMissing,

    #[error("Packet sink is missing")]
    PacketSinkMissing,

    #[error("Internal error: {0}")]
    Internal(Box<str>),
}
