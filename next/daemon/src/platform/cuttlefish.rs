// Copyright 2026 The Android Open Source Project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Cuttlefish (AOSP Soong) platform initialization policy.

use std::collections::HashMap;

use common::util::net::bind_tcp_loopback_with_ephemeral_fallback;
use device_actor::DeviceClient;
#[cfg(target_os = "linux")]
use packet_stream::{DualFdConfig, DualFdListener};
use packet_stream::{StreamAddress, Streams};
use tokio::task::JoinHandle;
#[cfg(target_os = "linux")]
use tracing::{error, info};

use crate::{args::Args, netsimd::RunResult};

const DEFAULT_ROOTCANAL_TEST_PORT: u16 = 7500;

#[derive(Debug, Default)]
pub(crate) struct PlatformTasks {
    test_task: Option<JoinHandle<()>>,
}

impl PlatformTasks {
    pub(crate) fn abort(&self) {
        if let Some(task) = &self.test_task {
            task.abort();
        }
    }
}

impl Drop for PlatformTasks {
    fn drop(&mut self) {
        self.abort();
    }
}

pub(crate) fn preflight_check(_args: &Args) -> Result<(), RunResult> {
    Ok(())
}

#[cfg(target_os = "linux")]
pub(crate) async fn init_transport_listeners(args: &Args, streams: &mut Streams) {
    if let Some(fd_startup_str) = &args.fd_startup_str {
        init_dualfd_listener(fd_startup_str, streams).await;
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn init_transport_listeners(
    _args: &Args,
    _streams: &mut Streams,
) -> std::future::Ready<()> {
    std::future::ready(())
}

#[cfg(target_os = "linux")]
async fn init_dualfd_listener(fd_startup_str: &str, streams: &mut Streams) {
    if fd_startup_str.is_empty() {
        return;
    }
    match serde_json::from_str::<DualFdConfig>(fd_startup_str) {
        Ok(config) => match DualFdListener::new(config).await {
            Ok(listener) => {
                if let Err(error) = streams.add_listener("netsim_dualfd", Box::new(listener)) {
                    error!(%error, "Failed to add DualFdListener");
                } else {
                    info!("Added DualFdListener");
                }
            }
            Err(error) => {
                error!(%error, "Failed to create DualFdListener");
            }
        },
        Err(error) => {
            error!(%error, "Failed to parse fd_startup_str JSON");
        }
    }
}

pub(crate) fn start_servers(
    args: &Args,
    instance_num: u16,
    device_client: &DeviceClient,
    listener_addresses: &mut HashMap<String, StreamAddress>,
    ini_data: &mut HashMap<String, String>,
) -> Result<PlatformTasks, RunResult> {
    let mut test_task = None;
    let is_explicit_test = args.test_port.is_some();
    let test_port =
        args.test_port.unwrap_or_else(|| DEFAULT_ROOTCANAL_TEST_PORT + instance_num - 1);
    if let Some(listener) = bind_tcp_loopback_with_ephemeral_fallback(
        test_port,
        is_explicit_test,
        "Rootcanal control server",
    )
    .map_err(|error| {
        crate::netsimd::init_error(format!(
            "Failed to bind requested Rootcanal control server port {test_port}: {error}"
        ))
    })? {
        let local_addr = listener.local_addr().map_err(crate::netsimd::init_error)?;
        listener_addresses.insert("netsim_rootcanal".to_string(), StreamAddress::Tcp(local_addr));
        ini_data.insert("test.port".to_string(), local_addr.port().to_string());
        // Spawn legacy Rootcanal control server for host-side test
        // runners (like pts-bot/mmi2grpc).
        test_task = Some(tokio::spawn(rootcanal_server::run(listener, device_client.clone())));
    }
    Ok(PlatformTasks { test_task })
}

#[must_use]
pub(crate) const fn should_start_tcp_listener(_resolved_tcp_port: Option<u16>) -> bool {
    true
}

#[must_use]
pub(crate) const fn default_test_beacons() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cuttlefish_defaults_enable_tcp_listener_and_test_beacons() {
        assert!(should_start_tcp_listener(None));
        assert!(should_start_tcp_listener(Some(8080)));
        assert!(default_test_beacons());
    }

    #[test]
    fn test_cuttlefish_preflight_check_succeeds() {
        let args = Args::default();
        assert!(preflight_check(&args).is_ok());
    }

    #[tokio::test]
    async fn test_cuttlefish_start_servers_binds_rootcanal_and_aborts_task() {
        let args = Args { test_port: Some(0), ..Default::default() };
        let (_runner, device_client) = device_actor::new();
        let mut listener_addresses = HashMap::new();
        let mut ini_data = HashMap::new();

        let tasks = start_servers(&args, 1, &device_client, &mut listener_addresses, &mut ini_data)
            .expect("start_servers with ephemeral port should succeed");

        let rootcanal_addr = listener_addresses
            .get("netsim_rootcanal")
            .expect("netsim_rootcanal listener must be registered");
        let bound_port = match rootcanal_addr {
            StreamAddress::Tcp(addr) => addr.port(),
            _ => panic!("Expected TCP address"),
        };
        assert_ne!(bound_port, 0);
        assert_eq!(ini_data.get("test.port"), Some(&bound_port.to_string()));

        tasks.abort();
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn test_cuttlefish_init_transport_listeners_ignores_empty_and_malformed_json() {
        let mut streams = Streams::new();
        let args_empty = Args { fd_startup_str: Some(String::new()), ..Default::default() };
        init_transport_listeners(&args_empty, &mut streams).await;
        assert!(streams.listener_address("netsim_dualfd").is_none());

        let args_malformed =
            Args { fd_startup_str: Some("invalid json".to_string()), ..Default::default() };
        init_transport_listeners(&args_malformed, &mut streams).await;
        assert!(streams.listener_address("netsim_dualfd").is_none());
    }
}
