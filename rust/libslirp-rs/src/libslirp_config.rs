// Copyright 2024 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::libslirp_sys;
use std::ffi::CString;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
use std::path::PathBuf;

// Rust SlirpConfig

pub struct SlirpConfig {
    pub version: u32,
    pub restricted: i32,
    pub in_enabled: bool,
    pub vnetwork: Ipv4Addr,
    pub vnetmask: Ipv4Addr,
    pub vhost: Ipv4Addr,
    pub in6_enabled: bool,
    pub vprefix_addr6: Ipv6Addr,
    pub vprefix_len: u8,
    pub vhost6: Ipv6Addr,
    pub vhostname: Option<String>,
    pub tftp_server_name: Option<String>,
    pub tftp_path: Option<PathBuf>,
    pub bootfile: Option<String>,
    pub vdhcp_start: Ipv4Addr,
    pub vnameserver: Ipv4Addr,
    pub vnameserver6: Ipv6Addr,
    pub vdnssearch: Vec<String>,
    pub vdomainname: Option<String>,
    pub if_mtu: usize,
    pub if_mru: usize,
    pub disable_host_loopback: bool,
    pub enable_emu: bool,
    pub outbound_addr: Option<SocketAddrV4>,
    pub outbound_addr6: Option<SocketAddrV6>,
    pub disable_dns: bool,
    pub disable_dhcp: bool,
    pub mfr_id: u32,
    pub oob_eth_addr: [u8; 6usize],
    pub http_proxy_on: bool,
    pub host_dns_count: usize,
    pub host_dns: [libslirp_sys::sockaddr_storage; 4usize],
}

impl Default for SlirpConfig {
    fn default() -> Self {
        SlirpConfig {
            version: 5,
            // No restrictions by default
            restricted: 0,
            in_enabled: true,
            // Private network address
            vnetwork: Ipv4Addr::new(10, 0, 2, 0),
            vnetmask: Ipv4Addr::new(255, 255, 255, 0),
            // Default host address
            vhost: Ipv4Addr::new(10, 0, 2, 2),
            // IPv6 disabled by default
            in6_enabled: true,
            vprefix_addr6: "fec0::".parse().unwrap(),
            vprefix_len: 64,
            vhost6: "fec0::2".parse().unwrap(),
            vhostname: None, // Some("slirp".to_string()),
            tftp_server_name: None,
            tftp_path: None,
            bootfile: None,
            // DHCP starting address
            vdhcp_start: Ipv4Addr::new(10, 0, 2, 16),
            // Public DNS server
            vnameserver: Ipv4Addr::new(10, 0, 2, 3),
            vnameserver6: "fec0::3".parse().unwrap(),
            vdnssearch: Vec::new(),
            vdomainname: None,
            // Ethernet MTU
            if_mtu: 0,
            // Ethernet MRU
            if_mru: 0,
            disable_host_loopback: false,
            enable_emu: false,
            outbound_addr: None,
            outbound_addr6: None,
            disable_dns: false,
            disable_dhcp: false,
            mfr_id: 0,
            oob_eth_addr: [0; 6usize],
            http_proxy_on: false,
            host_dns_count: 0,
            host_dns: [libslirp_sys::sockaddr_storage::default(); 4usize],
        }
    }
}

// Struct to hold a "C" SlirpConfig and the Rust storage that is
// referenced by SlirpConfig.
pub struct SlirpConfigs {
    pub c_slirp_config: libslirp_sys::SlirpConfig,

    // fields that hold the managed storage for "C" struct.
    c_bootfile: Option<CString>,
    c_tftp_server_name: Option<CString>,
    c_vdomainname: Option<CString>,
    c_vhostname: Option<CString>,
    c_tftp_path: Option<CString>,
    // TODO: add other fields
}

impl SlirpConfigs {
    pub fn new(config: &SlirpConfig) -> SlirpConfigs {
        let as_cstring =
            |s: &Option<String>| s.as_ref().and_then(|s| CString::new(s.as_bytes()).ok());
        let c_tftp_path = config
            .tftp_path
            .as_ref()
            .and_then(|s| CString::new(s.to_string_lossy().into_owned()).ok());
        let c_vhostname = as_cstring(&config.vhostname);
        let c_tftp_server_name = as_cstring(&config.tftp_server_name);
        let c_bootfile = as_cstring(&config.bootfile);
        let c_vdomainname = as_cstring(&config.vdomainname);

        // Convert to a ptr::null() or a raw ptr to managed
        // memory. Whenever storing a ptr in "C" Struct using `as_ptr`
        // this code must have a Rust member is `SlirpConfigs` that
        // holds the storage.
        let as_ptr = |p: &Option<CString>| p.as_ref().map_or(std::ptr::null(), |s| s.as_ptr());

        let c_slirp_config = libslirp_sys::SlirpConfig {
            version: config.version,
            restricted: config.restricted,
            in_enabled: config.in_enabled,
            vnetwork: config.vnetwork.into(),
            vnetmask: config.vnetmask.into(),
            vhost: config.vhost.into(),
            in6_enabled: config.in6_enabled,
            vprefix_addr6: config.vprefix_addr6.into(),
            vprefix_len: config.vprefix_len,
            vhost6: config.vhost6.into(),
            vhostname: as_ptr(&c_vhostname),
            tftp_server_name: as_ptr(&c_tftp_server_name),
            tftp_path: as_ptr(&c_tftp_path),
            bootfile: as_ptr(&c_bootfile),
            vdhcp_start: config.vdhcp_start.into(),
            vnameserver: config.vnameserver.into(),
            vnameserver6: config.vnameserver6.into(),
            // TODO: add field
            vdnssearch: std::ptr::null_mut(),
            vdomainname: as_ptr(&c_vdomainname),
            if_mtu: config.if_mtu,
            if_mru: config.if_mru,
            disable_host_loopback: config.disable_host_loopback,
            enable_emu: config.enable_emu,
            // TODO: add field
            outbound_addr: std::ptr::null_mut(),
            // TODO: add field
            outbound_addr6: std::ptr::null_mut(),
            disable_dns: config.disable_dns,
            disable_dhcp: config.disable_dhcp,
            mfr_id: config.mfr_id,
            oob_eth_addr: config.oob_eth_addr,
            http_proxy_on: config.http_proxy_on,
            host_dns_count: config.host_dns_count,
            host_dns: config.host_dns,
        };

        // Return the "C" struct and Rust members holding the storage
        // referenced by the "C" struct.
        SlirpConfigs {
            c_slirp_config,
            c_vhostname,
            c_tftp_server_name,
            c_bootfile,
            c_vdomainname,
            c_tftp_path,
        }
    }
}
