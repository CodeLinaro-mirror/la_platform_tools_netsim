// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SlirpError {
    #[error("LibSlirp not initialized")]
    NotInitialized,
    #[error("Internal error: {0}")]
    Internal(String),
}
