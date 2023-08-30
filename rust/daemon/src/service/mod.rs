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
use crate::bluetooth::advertise_settings as ble_advertise_settings;
use crate::captures;
use crate::captures::captures_handler::clear_pcap_files;
use crate::config::{get_dev, set_dev};
use crate::devices::devices_handler::is_shutdown_time;
use crate::ffi::{get_netsim_ini_file_path_cxx, run_grpc_server_cxx};
use crate::http_server::server::run_http_server;
use crate::resource;
use crate::transport::socket::run_socket_transport;
use crate::wifi as wifi_facade;
use log::{error, info, warn};
use netsim_common::util::ini_file::IniFile;
use std::env;
use std::time::Duration;

/// Module to control startup, run, and cleanup netsimd services.

const INACTIVITY_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

pub struct ServiceParams {
    fd_startup_str: String,
    no_cli_ui: bool,
    no_web_ui: bool,
    hci_port: u16,
    instance_num: u16,
    dev: bool,
    vsock: u16,
}

impl ServiceParams {
    pub fn new(
        fd_startup_str: String,
        no_cli_ui: bool,
        no_web_ui: bool,
        hci_port: u16,
        instance_num: u16,
        dev: bool,
        vsock: u16,
    ) -> Self {
        ServiceParams { fd_startup_str, no_cli_ui, no_web_ui, hci_port, instance_num, dev, vsock }
    }
}

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
        if clear_pcap_files() {
            info!("netsim generated pcap files in temp directory has been removed.");
        }
        set_dev(self.service_params.dev);

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
            self.service_params.vsock,
        );
        if grpc_server.is_null() {
            error!("Failed to run netsimd because unable to start grpc server");
            return;
        }

        let forge_job = netsim_grpc_port != 0;

        // forge and no_web_ui disables the web server
        let mut web_port: Option<u16> = None;
        if !forge_job && !self.service_params.no_web_ui {
            web_port = Some(run_http_server(self.service_params.instance_num));
        }

        // Write to netsim.ini file
        let filepath = get_netsim_ini_file_path_cxx(self.service_params.instance_num);
        let mut ini_file = IniFile::new(filepath.to_string());
        if let Some(num) = web_port {
            ini_file.insert("web.port", &num.to_string());
        }
        ini_file.insert("grpc.port", &grpc_server.get_grpc_port().to_string());
        if let Err(err) = ini_file.write() {
            error!("{err:?}");
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

pub fn new_test_beacon(idx: u32) {
    use crate::devices::devices_handler::create_device;
    use netsim_proto::common::ChipKind;
    use netsim_proto::frontend::CreateDeviceRequest;
    use netsim_proto::model::chip::bluetooth_beacon::{
        AdvertiseData as AdvertiseDataProto, AdvertiseSettings as AdvertiseSettingsProto,
    };
    use netsim_proto::model::chip_create::{
        BluetoothBeaconCreate as BluetoothBeaconCreateProto, Chip as ChipProto,
    };
    use netsim_proto::model::ChipCreate as ChipCreateProto;
    use netsim_proto::model::DeviceCreate as DeviceCreateProto;
    use protobuf::MessageField;
    use protobuf_json_mapping::print_to_string;

    let beacon_proto = BluetoothBeaconCreateProto {
        address: format!("be:ac:01:be:ef:{:02x}", idx),
        settings: MessageField::some(AdvertiseSettingsProto {
            interval: Some(
                ble_advertise_settings::AdvertiseMode::new(Duration::from_millis(1280))
                    .try_into()
                    .unwrap(),
            ),
            scannable: true,
            ..Default::default()
        }),
        adv_data: MessageField::some(AdvertiseDataProto {
            include_device_name: true,
            include_tx_power_level: true,
            manufacturer_data: vec![1u8, 2, 3, 4],
            ..Default::default()
        }),
        ..Default::default()
    };

    let chip_proto = ChipCreateProto {
        name: format!("beacon-{idx}"),
        kind: ChipKind::BLUETOOTH_BEACON.into(),
        chip: Some(ChipProto::BleBeacon(beacon_proto)),
        ..Default::default()
    };

    let device_proto = DeviceCreateProto {
        name: format!("device-{idx}"),
        chips: vec![chip_proto],
        ..Default::default()
    };

    let request =
        CreateDeviceRequest { device: MessageField::some(device_proto), ..Default::default() };

    if let Err(err) = create_device(&print_to_string(&request).unwrap()) {
        warn!("Failed to create beacon device {idx}: {err}");
    }
}
