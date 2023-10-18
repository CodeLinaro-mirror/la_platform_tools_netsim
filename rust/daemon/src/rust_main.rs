// Copyright 2023 Google LLC
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

use clap::Parser;
use log::warn;
use log::{error, info};
use netsim_common::util::os_utils::remove_netsim_ini;
use netsim_common::util::zip_artifact::zip_artifacts;

use crate::bluetooth as bluetooth_facade;
use crate::config_file;
use crate::wifi as wifi_facade;
use netsim_common::util::netsim_logger;

use crate::args::NetsimdArgs;
use crate::ffi::ffi_util;
use crate::service::{Service, ServiceParams};
#[cfg(feature = "cuttlefish")]
use netsim_common::util::os_utils::get_server_address;
use netsim_proto::config::Config;
use std::ffi::{c_char, c_int};

/// Wireless network simulator for android (and other) emulated devices.
///
/// # Safety
///
/// The file descriptors passed in `NetsimdArgs::fd_startup_str` must remain valid and open for as
/// long as the program runs.
#[no_mangle]
pub unsafe extern "C" fn rust_main(argc: c_int, argv: *const *const c_char) {
    ffi_util::set_up_crash_report();
    netsim_logger::init("netsimd");
    let netsimd_args = get_netsimd_args(argc, argv);
    run_netsimd_with_args(netsimd_args);
}

#[allow(unused)]
fn get_netsimd_args(argc: c_int, argv: *const *const c_char) -> NetsimdArgs {
    #[cfg(feature = "cuttlefish")]
    {
        // TODO: Use NetsimdArgs::parse() after netsimd binary is built with netsimd.rs.
        // In linux arm64 in aosp-main, it can't access CLI arguments by std::env::args() with netsimd.cc wrapper.
        let argv: Vec<_> = (0..argc)
            .map(|i|
                // SAFETY: argc and argv will remain valid as long as the program runs.
                unsafe {
                    std::ffi::CStr::from_ptr(*argv.add(i as usize)).to_str().unwrap().to_owned()
                })
            .collect();
        NetsimdArgs::parse_from(argv)
    }
    #[cfg(not(feature = "cuttlefish"))]
    NetsimdArgs::parse()
}

fn run_netsimd_with_args(args: NetsimdArgs) {
    // Redirect stdout and stderr to files only if netsimd is not invoked
    // by Cuttlefish. Some Cuttlefish builds fail when writing logs to files.
    #[cfg(not(feature = "cuttlefish"))]
    if !args.logtostderr {
        cxx::let_cxx_string!(netsimd_temp_dir = netsim_common::system::netsimd_temp_dir_string());
        ffi_util::redirect_std_stream(&netsimd_temp_dir);
    }

    match args.connector_instance {
        #[cfg(feature = "cuttlefish")]
        Some(connector_instance) => run_netsimd_connector(args, connector_instance),
        _ => run_netsimd_primary(args),
    }
}

/// Forwards packets to another netsim daemon.
#[cfg(feature = "cuttlefish")]
fn run_netsimd_connector(args: NetsimdArgs, instance: u16) {
    if args.fd_startup_str.is_none() {
        error!("Failed to start netsimd forwarder, missing `-s` arg");
        return;
    }
    let fd_startup = args.fd_startup_str.unwrap();

    let mut server: Option<String> = None;
    // Attempts multiple time for fetching netsim.ini
    for second in [1, 2, 4, 8, 0] {
        server = get_server_address(instance);
        if server.is_some() {
            break;
        } else {
            warn!("Unable to find ini file for instance {}, retrying", instance);
            std::thread::sleep(std::time::Duration::from_secs(second));
        }
    }
    if server.is_none() {
        error!("Failed to run netsimd connector");
        return;
    }
    let server = server.unwrap();
    // TODO: Make this function returns Result to use `?` instead of unwrap().
    info!("Starting in Connector mode to {}", server.as_str());
    crate::transport::fd::run_fd_connector(&fd_startup, server.as_str())
        .map_err(|e| error!("Failed to run fd connector: {}", e))
        .unwrap();
}

fn run_netsimd_primary(args: NetsimdArgs) {
    let fd_startup_str = args.fd_startup_str.unwrap_or_default();
    let instance_num = ffi_util::get_instance(args.instance.unwrap_or_default());
    let hci_port: u16 =
        ffi_util::get_hci_port(args.hci_port.unwrap_or_default(), instance_num).try_into().unwrap();

    #[cfg(feature = "cuttlefish")]
    if fd_startup_str.is_empty() {
        warn!("Failed to start netsim daemon because fd startup flag `-s` is empty");
        return;
    }

    if ffi_util::is_netsimd_alive(instance_num) {
        warn!("Failed to start netsim daemon because a netsim daemon is already running");
        return;
    }

    let mut config = Config::new();
    if let Some(filename) = args.config {
        match config_file::new_from_file(&filename) {
            Ok(config_from_file) => {
                info!("Using config in {}", config);
                config = config_from_file;
            }
            Err(e) => {
                error!("Skipping config in {}: {:?}", filename, e);
            }
        }
    }

    let service_params = ServiceParams::new(
        fd_startup_str,
        args.no_cli_ui,
        args.no_web_ui,
        args.pcap,
        args.disable_address_reuse,
        hci_port,
        instance_num,
        args.dev,
        args.vsock.unwrap_or_default(),
    );

    // SAFETY: The caller guaranteed that the file descriptors in `fd_startup_str` would remain
    // valid and open for as long as the program runs.
    let service = unsafe { Service::new(service_params) };
    service.set_up();

    bluetooth_facade::bluetooth_start(&config.bluetooth, instance_num);
    wifi_facade::wifi_start(&config.wifi);

    service.run();

    // Once service.run is complete, delete the netsim ini file
    // and zip all artifacts
    remove_netsim_ini(instance_num);
    if let Err(err) = zip_artifacts() {
        error!("Failed to zip artifacts: {err:?}");
    }
}
