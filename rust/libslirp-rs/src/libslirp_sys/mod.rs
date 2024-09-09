//  Copyright 2024 Google, Inc.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at:
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
// TODO(b/203002625) - since rustc 1.53, bindgen causes UB warnings
// Remove this once bindgen figures out how to do this correctly
#![allow(deref_nullptr)]

#[cfg(target_os = "linux")]
include!("linux/bindings.rs");

#[cfg(target_os = "macos")]
include!("macos/bindings.rs");

#[cfg(target_os = "windows")]
include!("windows/bindings.rs");

use std::convert::From;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};

impl Default for sockaddr_storage {
    #[cfg(target_os = "macos")]
    fn default() -> Self {
        sockaddr_storage {
            ss_len: 0,
            ss_family: 0,
            __ss_pad1: [0i8; 6],
            __ss_align: 0,
            __ss_pad2: [0i8; 112],
        }
    }

    #[cfg(target_os = "linux")]
    fn default() -> Self {
        sockaddr_storage { ss_family: 0, __ss_padding: [0i8; 118], __ss_align: 0 }
    }

    #[cfg(target_os = "windows")]
    fn default() -> Self {
        sockaddr_storage { ss_family: 0, __ss_pad1: [0i8; 6], __ss_align: 0, __ss_pad2: [0i8; 112] }
    }
}
// Type for libslirp poll bitfield mask SLIRP_POLL_nnn

#[cfg(target_os = "linux")]
pub type SlirpPollType = _bindgen_ty_7;

#[cfg(target_os = "macos")]
pub type SlirpPollType = _bindgen_ty_1;

#[cfg(target_os = "windows")]
pub type SlirpPollType = _bindgen_ty_5;

impl From<Ipv4Addr> for in_addr {
    #[cfg(target_os = "macos")]
    fn from(item: Ipv4Addr) -> Self {
        in_addr { s_addr: std::os::raw::c_uint::to_be(item.into()) }
    }

    #[cfg(target_os = "linux")]
    fn from(item: Ipv4Addr) -> Self {
        in_addr { s_addr: u32::to_be(item.into()) }
    }

    #[cfg(target_os = "windows")]
    fn from(item: Ipv4Addr) -> Self {
        in_addr {
            S_un: in_addr__bindgen_ty_1 { S_addr: std::os::raw::c_ulong::to_be(item.into()) },
        }
    }
}

impl From<Ipv6Addr> for in6_addr {
    #[cfg(target_os = "macos")]
    fn from(item: Ipv6Addr) -> Self {
        in6_addr { __u6_addr: in6_addr__bindgen_ty_1 { __u6_addr8: item.octets() } }
    }

    #[cfg(target_os = "linux")]
    fn from(item: Ipv6Addr) -> Self {
        in6_addr { __in6_u: in6_addr__bindgen_ty_1 { __u6_addr8: item.octets() } }
    }

    #[cfg(target_os = "windows")]
    fn from(item: Ipv6Addr) -> Self {
        in6_addr { u: in6_addr__bindgen_ty_1 { Byte: item.octets() } }
    }
}

impl From<SocketAddrV4> for sockaddr_in {
    #[cfg(target_os = "macos")]
    fn from(item: SocketAddrV4) -> Self {
        sockaddr_in {
            sin_len: 16u8,
            sin_family: AF_INET as u8,
            sin_port: item.port().to_be(),
            sin_addr: (*item.ip()).into(),
            sin_zero: [0; 8],
        }
    }
    #[cfg(target_os = "linux")]
    fn from(item: SocketAddrV4) -> Self {
        sockaddr_in {
            sin_family: AF_INET as u16,
            sin_port: item.port().to_be(),
            sin_addr: (*item.ip()).into(),
            sin_zero: [0; 8],
        }
    }
    #[cfg(target_os = "windows")]
    fn from(item: SocketAddrV4) -> Self {
        sockaddr_in {
            sin_family: AF_INET as u16,
            sin_port: item.port().to_be(),
            sin_addr: (*item.ip()).into(),
            sin_zero: [0; 8],
        }
    }
}

impl From<SocketAddrV6> for sockaddr_in6 {
    #[cfg(target_os = "windows")]
    fn from(item: SocketAddrV6) -> Self {
        sockaddr_in6 {
            sin6_addr: (*item.ip()).into(),
            sin6_family: AF_INET6 as u16,
            sin6_port: item.port().to_be(),
            sin6_flowinfo: item.flowinfo(),
            __bindgen_anon_1: sockaddr_in6__bindgen_ty_1 { sin6_scope_id: item.scope_id() },
        }
    }
    #[cfg(target_os = "macos")]
    fn from(item: SocketAddrV6) -> Self {
        sockaddr_in6 {
            sin6_len: 16,
            sin6_addr: (*item.ip()).into(),
            sin6_family: AF_INET6 as u8,
            sin6_port: item.port().to_be(),
            sin6_flowinfo: item.flowinfo(),
            sin6_scope_id: item.scope_id(),
        }
    }
    #[cfg(target_os = "linux")]
    fn from(item: SocketAddrV6) -> Self {
        sockaddr_in6 {
            sin6_addr: (*item.ip()).into(),
            sin6_family: AF_INET6 as u16,
            sin6_port: item.port().to_be(),
            sin6_flowinfo: item.flowinfo(),
            sin6_scope_id: item.scope_id(),
        }
    }
}
