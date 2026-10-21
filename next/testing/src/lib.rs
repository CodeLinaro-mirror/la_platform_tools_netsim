// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "testing")]
mod logger;
#[cfg(feature = "testing")]
pub use logger::setup;

#[cfg(feature = "testing")]
mod mocks;
#[cfg(feature = "testing")]
pub use mocks::{mock_sink, mock_stream};
