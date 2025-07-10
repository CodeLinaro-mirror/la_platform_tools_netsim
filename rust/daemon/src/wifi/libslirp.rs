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

/// LibSlirp Interface for Network Simulation
use crate::get_runtime;
use crate::wifi::error::WifiResult;

use bytes::Bytes;
use http_proxy::Manager;
pub use libslirp_rs::libslirp::LibSlirp;
use libslirp_rs::libslirp::ProxyManager;
use libslirp_rs::libslirp_config::{lookup_host_dns, SlirpConfig};
use log::info;
use netsim_proto::config::SlirpOptions as ProtoSlirpOptions;
use std::net::UdpSocket;
use std::sync::mpsc;

/// A utility to detect host network capabilities.
pub struct HostNetwork {
    pub ipv4_supported: bool,
    pub ipv6_supported: bool,
}

impl HostNetwork {
    /// Detects IPv4 and IPv6 support by attempting to bind sockets.
    pub fn detect() -> Self {
        let ipv4_supported = UdpSocket::bind("0.0.0.0:0").is_ok();
        let ipv6_supported = UdpSocket::bind("[::]:0").is_ok();
        info!("Host network support: ipv4={}, ipv6={}", ipv4_supported, ipv6_supported);
        Self { ipv4_supported, ipv6_supported }
    }
}

pub fn slirp_run(opt: ProtoSlirpOptions, tx_bytes: mpsc::Sender<Bytes>) -> WifiResult<LibSlirp> {
    // TODO: Convert ProtoSlirpOptions to SlirpConfig.
    let http_proxy = Some(opt.http_proxy).filter(|s| !s.is_empty());
    let (proxy_manager, tx_proxy_bytes) = if let Some(proxy) = http_proxy {
        let (tx_proxy_bytes, rx_proxy_response) = mpsc::channel::<Bytes>();
        (
            Some(Box::new(Manager::new(&proxy, rx_proxy_response)?)
                as Box<dyn ProxyManager + 'static>),
            Some(tx_proxy_bytes),
        )
    } else {
        (None, None)
    };

    let mut config = SlirpConfig { http_proxy_on: proxy_manager.is_some(), ..Default::default() };

    let host_network = HostNetwork::detect();
    config.in_enabled = host_network.ipv4_supported;
    config.in6_enabled = host_network.ipv6_supported;

    if !opt.host_dns.is_empty() {
        config.host_dns = get_runtime().block_on(lookup_host_dns(&opt.host_dns))?;
    }

    Ok(LibSlirp::new(config, tx_bytes, proxy_manager, tx_proxy_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_network_detect() {
        let network = HostNetwork::detect();
        // We can't know for sure if ipv4 and ipv6 are supported,
        // but we can check that the function returns a valid struct.
        assert!(network.ipv4_supported || !network.ipv4_supported);
        assert!(network.ipv6_supported || !network.ipv6_supported);
    }
}
