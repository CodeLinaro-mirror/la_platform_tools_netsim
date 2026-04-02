// Copyright 2023 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Capture Resource codebase

pub mod capture;
pub mod captures_handler;
pub mod pcap_util;
/// A mime type for pcap file in HTTP headers. [application/vnd.tcpdump.pcap]
pub const PCAP_MIME_TYPE: &str = "application/vnd.tcpdump.pcap";

pub use captures_handler::controller_to_host;
pub use captures_handler::host_to_controller;
