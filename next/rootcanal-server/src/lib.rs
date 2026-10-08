// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! A Rust rewrite of the C++ RootCanal test/control channel server, emulating
//! its protocol by translating console commands into Netsim device actor
//! updates.

mod server;

pub use server::run;
