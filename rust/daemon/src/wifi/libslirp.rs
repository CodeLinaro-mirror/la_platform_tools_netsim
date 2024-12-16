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
use bytes::Bytes;
use http_proxy::Manager;
pub use libslirp_rs::libslirp::LibSlirp;
use libslirp_rs::libslirp::ProxyManager;
use libslirp_rs::libslirp_config::{lookup_host_dns, SlirpConfig};
use netsim_proto::config::SlirpOptions as ProtoSlirpOptions;
use std::sync::mpsc;
use tokio::runtime::Runtime;

pub fn slirp_run(
    opt: ProtoSlirpOptions,
    tx_bytes: mpsc::Sender<Bytes>,
) -> anyhow::Result<LibSlirp> {
    // TODO: Convert ProtoSlirpOptions to SlirpConfig.
    let http_proxy = Some(opt.http_proxy).filter(|s| !s.is_empty());
    let proxy_manager = if let Some(proxy) = http_proxy {
        Some(Box::new(Manager::new(&proxy)?) as Box<dyn ProxyManager + 'static>)
    } else {
        None
    };

    let mut config = SlirpConfig { http_proxy_on: proxy_manager.is_some(), ..Default::default() };

    if !opt.host_dns.is_empty() {
        let rt = Runtime::new().unwrap();
        config.host_dns = rt.block_on(lookup_host_dns(&opt.host_dns))?;
    }

    Ok(LibSlirp::new(config, tx_bytes, proxy_manager))
}
