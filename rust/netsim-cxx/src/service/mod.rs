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

use crate::bluetooth as bluetooth_facade;
use crate::captures;
use crate::captures::handlers::clear_pcap_files;
use crate::config::get_dev;
use crate::devices::devices_handler::is_shutdown_time;
use crate::ffi::run_grpc_server_cxx;
use crate::http_server::run_http_server;
use crate::resource;
use crate::transport::socket::run_socket_transport;
use crate::wifi as wifi_facade;
use log::{error, info, warn};
use netsim_common::util::netsim_logger;
use std::env;

/// Module to control startup, run, and cleanup netsimd services.

const INACTIVITY_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

pub struct ServiceParams {
    fd_startup_str: String,
    no_cli_ui: bool,
    no_web_ui: bool,
    hci_port: u16,
    instance_num: u16,
    dev: bool,
}

// TODO: Replace Run() in server.cc.

pub struct Service {
    // netsimd states, like device resource.
    service_params: ServiceParams,
}

impl Service {
    /// # Safety
    ///
    /// The file descriptors in `service_params.fd_startup_str` must be valid and open, and must
    /// remain so for as long as the `Service` exists.
    pub unsafe fn new(service_params: ServiceParams) -> Service {
        Service { service_params }
    }

    /// Sets up the states for netsimd.
    pub fn set_up(&self) {
        netsim_logger::init("netsimd");
        if clear_pcap_files() {
            info!("netsim generated pcap files in temp directory has been removed.");
        }

        // Start all the subscribers for events
        let events_rx = resource::clone_events().lock().unwrap().subscribe();
        captures::capture::spawn_capture_event_subscriber(events_rx);

        bluetooth_facade::bluetooth_start(self.service_params.instance_num);
        wifi_facade::wifi_start();
    }

    /// Runs the netsimd services.
    pub fn run(&self) {
        // TODO: run grpc server.

        if !self.service_params.fd_startup_str.is_empty() {
            // SAFETY: When the `Service` was constructed by `Service::new` the caller guaranteed
            // that the file descriptors in `service_params.fd_startup_str` would remain valid and
            // open.
            unsafe {
                use crate::transport::fd::run_fd_transport;
                run_fd_transport(&self.service_params.fd_startup_str);
            }
        }
        // Environment variable "NETSIM_GRPC_PORT" is set in google3 forge jobs. If set:
        // 1. Use the fixed port for grpc server.
        // 2. Don't start http server.
        let netsim_grpc_port =
            env::var("NETSIM_GRPC_PORT").map(|val| val.parse::<u32>().unwrap_or(0)).unwrap_or(0);
        let grpc_server = run_grpc_server_cxx(
            netsim_grpc_port,
            self.service_params.no_cli_ui,
            self.service_params.instance_num,
        );
        if grpc_server.is_null() {
            error!("Failed to run netsimd because unable to start grpc server");
            return;
        }

        let forge_job = netsim_grpc_port != 0;

        // forge and no_web_ui disables the web server
        if !forge_job && !self.service_params.no_web_ui {
            run_http_server(self.service_params.instance_num);
        }

        // Run the socket server.
        run_socket_transport(self.service_params.hci_port);

        if get_dev() {
            new_test_beacon(0);
            new_test_beacon(1);
        }

        loop {
            std::thread::sleep(INACTIVITY_CHECK_INTERVAL);
            if is_shutdown_time() {
                grpc_server.shut_down();
                info!("Netsim has been shutdown due to inactivity.");
                break;
            }
        }
    }
}

// For cxx.
/// # Safety
///
/// The file descriptors in `fd_startup_str` must be valid and open, and must remain so for as long
/// as the returned `Service` exists.
pub unsafe fn create_service(
    fd_startup_str: String,
    no_cli_ui: bool,
    no_web_ui: bool,
    hci_port: u16,
    instance_num: u16,
    dev: bool,
) -> Box<Service> {
    let service_params =
        ServiceParams { fd_startup_str, no_cli_ui, no_web_ui, hci_port, instance_num, dev };
    // SAFETY: The caller guarandeed that the file descriptors in `fd_startup_str` would remain
    // valid and open for as long as the `Service` exists.
    Box::new(unsafe { Service::new(service_params) })
}

pub fn new_test_beacon(idx: u32) {
    use crate::bluetooth::new_beacon;
    use frontend_proto::model::chip::bluetooth_beacon::{
        AdvertiseData as AdvertiseDataProto, AdvertiseSettings as AdvertiseSettingsProto,
    };
    use frontend_proto::model::chip_create::{
        BluetoothBeaconCreate as BluetoothBeaconCreateProto, Chip as ChipProto,
    };
    use frontend_proto::model::{ChipCreate as ChipCreateProto, DeviceCreate as DeviceCreateProto};
    use protobuf::MessageField;

    if let Err(err) = new_beacon(&DeviceCreateProto {
        name: format!("test-beacon-device-{idx}"),
        chips: vec![ChipCreateProto {
            name: format!("test-beacon-chip-{idx}"),
            chip: Some(ChipProto::BleBeacon(BluetoothBeaconCreateProto {
                address: format!("00:00:00:00:00:{:x}", idx),
                settings: MessageField::some(AdvertiseSettingsProto {
                    tx_power_level: 0,
                    interval: 1280,
                    ..Default::default()
                }),
                adv_data: MessageField::some(AdvertiseDataProto {
                    include_device_name: true,
                    include_tx_power_level: true,
                    manufacturer_data: vec![1u8, 2, 3, 4],
                    ..Default::default()
                }),
                ..Default::default()
            })),
            ..Default::default()
        }],
        ..Default::default()
    }) {
        warn!("Failed to create beacon device {idx}: {err}");
    }
}
