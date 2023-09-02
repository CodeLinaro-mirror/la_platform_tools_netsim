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
use log::{error, info, warn};

use netsim_common::util::netsim_logger;

use crate::args::NetsimdArgs;
use crate::ffi::ffi_util;
use crate::service::{Service, ServiceParams};
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

    match args.primary_instance {
        Some(primary_instance) => run_netsimd_forwarder(args, primary_instance),
        None => run_netsimd_primary(args),
    }
}

fn run_netsimd_forwarder(args: NetsimdArgs, primary_instance: u16) {
    if args.fd_startup_str.is_none() {
        error!("Failed to start netsimd forwarder, missing `-s` arg");
        return;
    }
    if !ffi_util::is_netsimd_alive(primary_instance) {
        error!("Failed to start netsimd forwarder, no primary at {}", primary_instance);
        return;
    }
    info!("Starting netsim daemon in forwarding mode");
}

fn run_netsimd_primary(netsimd_args: NetsimdArgs) {
    let fd_startup_str = netsimd_args.fd_startup_str.unwrap_or_default();
    let no_cli_ui = netsimd_args.no_cli_ui;
    let no_web_ui = netsimd_args.no_web_ui;
    let pcap = netsimd_args.pcap;
    let disable_address_reuse = netsimd_args.disable_address_reuse;
    let instance_num = ffi_util::get_instance(netsimd_args.instance.unwrap_or_default());
    let hci_port: u16 =
        ffi_util::get_hci_port(netsimd_args.hci_port.unwrap_or_default(), instance_num)
            .try_into()
            .unwrap();
    let dev = netsimd_args.dev;
    let vsock = netsimd_args.vsock.unwrap_or_default();

    #[cfg(feature = "cuttlefish")]
    if fd_startup_str.is_empty() {
        warn!("Failed to start netsim daemon because fd startup flag `-s` is empty");
        return;
    }

    if ffi_util::is_netsimd_alive(instance_num) {
        warn!("Failed to start netsim daemon because a netsim daemon is already running");
        return;
    }
    let service_params = ServiceParams::new(
        fd_startup_str,
        no_cli_ui,
        no_web_ui,
        pcap,
        disable_address_reuse,
        hci_port,
        instance_num,
        dev,
        vsock,
    );

    // SAFETY: The caller guaranteed that the file descriptors in `fd_startup_str` would remain
    // valid and open for as long as the program runs.
    let service = unsafe { Service::new(service_params) };
    service.set_up();
    service.run();
}
