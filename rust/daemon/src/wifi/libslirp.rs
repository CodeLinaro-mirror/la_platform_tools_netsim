// Copyright 2024 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

/// LibSlirp Interface for Network Simulation
use crate::get_runtime;
use crate::wifi::error::WifiResult;

use bytes::Bytes;
use http_proxy::Manager;
pub use libslirp_rs::libslirp::LibSlirp;
use libslirp_rs::libslirp::ProxyManager;
use libslirp_rs::libslirp_config::{lookup_host_dns, SlirpConfig};
use netsim_proto::config::SlirpOptions as ProtoSlirpOptions;
use std::sync::mpsc;

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

    if !opt.host_dns.is_empty() {
        config.host_dns = get_runtime().block_on(lookup_host_dns(&opt.host_dns))?;
    }

    Ok(LibSlirp::new(config, tx_bytes, proxy_manager, tx_proxy_bytes))
}
