// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#[cfg(not(windows))]
mod dns_test;
mod lifecycle_test;
#[cfg(not(windows))]
mod udp_test;
mod world;
