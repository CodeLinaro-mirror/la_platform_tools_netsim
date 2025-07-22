//
// Copyright (C) 2025 The Android Open Source Project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

//! This module defines the error types for the ble-advertisers crate.

use thiserror::Error;

/// The error type for the ble-advertisers crate.
#[derive(Error, Debug)]
pub enum Error {
    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The advertising or scan response data exceeds the 31-byte limit.
    #[error("Data exceeds 31-byte limit")]
    DataTooLong,

    /// An invalid input was provided.
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Io(l), Self::Io(r)) => l.kind() == r.kind(),
            (Self::DataTooLong, Self::DataTooLong) => true,
            (Self::InvalidInput(l), Self::InvalidInput(r)) => l == r,
            _ => false,
        }
    }
}
