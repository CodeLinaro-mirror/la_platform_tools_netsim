// Copyright 2023 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// [cfg(test)] gets compiled during local Rust unit tests
// [cfg(not(test))] avoids getting compiled during local Rust unit tests

pub(crate) mod error;
pub(crate) mod frame;
#[cfg_attr(feature = "cuttlefish", path = "hostapd_cf.rs")]
pub(crate) mod hostapd;
pub(crate) mod hwsim_attr_set;
#[cfg_attr(feature = "cuttlefish", path = "libslirp_cf.rs")]
pub(crate) mod libslirp;
#[cfg(not(feature = "cuttlefish"))]
pub(crate) mod mdns_forwarder;
pub(crate) mod medium;
pub(crate) mod radiotap;
pub(crate) mod stats;
