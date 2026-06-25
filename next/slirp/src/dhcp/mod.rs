// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

mod dhcp_impl;
pub use dhcp_impl::DhcpManager;
#[cfg(test)]
pub(crate) use dhcp_impl::{DHCP_MAGIC_COOKIE, DhcpMessageType, DhcpPacket};
