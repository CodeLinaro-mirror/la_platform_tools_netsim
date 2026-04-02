// Copyright 2023 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// [cfg(test)] gets compiled during local Rust unit tests
// [cfg(not(test))] avoids getting compiled during local Rust unit tests

#![allow(unused)]
mod beacon;
#[cfg(test)]
mod mocked;
pub(crate) use self::beacon::*;
#[cfg(test)]
pub(crate) use self::mocked::*;
pub(crate) mod advertise_data;
pub(crate) mod advertise_settings;
pub(crate) mod chip;
