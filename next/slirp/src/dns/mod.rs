// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

mod dns_impl;
pub use dns_impl::{
    DnsProxy, DnsQueryResult, create_dns_search_option, discover_host_dns_servers,
    encode_dns_search_list, parse_resolv_conf_content,
};

#[cfg(test)]
mod tests;
