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

//! Netsim cxx libraries.

#![allow(dead_code)]

mod bluetooth;
pub mod captures;
mod config;
mod devices;
mod http_server;
mod ranging;
mod resource;
mod service;
mod system;
mod transport;
mod uwb;
mod version;
mod wifi;

// This feature is enabled only for CMake builds
#[cfg(feature = "local_ssl")]
mod openssl;

use std::pin::Pin;

use bluetooth::chip::{
    create_add_rust_device_result, AddRustDeviceResult, RustBluetoothChipCallbacks,
};
use cxx::let_cxx_string;
use ffi::CxxServerResponseWriter;
use http_server::http_request::StrHeaders;
use http_server::server_response::ServerResponseWritable;

use crate::transport::dispatcher::handle_response;
use crate::transport::fd::run_fd_transport;
use crate::transport::grpc::{register_grpc_transport, unregister_grpc_transport};
use crate::transport::socket::run_socket_transport;

use crate::captures::handlers::{
    clear_pcap_files, handle_capture_cxx, handle_packet_request, handle_packet_response,
    update_captures,
};
use crate::config::{get_dev, set_dev};
use crate::devices::devices_handler::{
    add_chip_cxx, get_distance_cxx, handle_device_cxx, is_shutdown_time_cxx, remove_chip_cxx,
    AddChipResultCxx,
};
use crate::http_server::run_http_server;
use crate::ranging::*;
use crate::service::{create_service, Service};
use crate::system::netsimd_temp_dir_string;
use crate::uwb::facade::*;
use crate::version::*;

#[cxx::bridge(namespace = "netsim")]
mod ffi {

    extern "Rust" {
        #[cxx_name = "RunSocketTransport"]
        fn run_socket_transport(hci_port: u16);

        #[cxx_name = "RunFdTransport"]
        fn run_fd_transport(startup_json: &String);

        #[cxx_name = "RunHttpServer"]
        fn run_http_server();

        // Config
        #[cxx_name = "GetDev"]
        #[namespace = "netsim::config"]
        fn get_dev() -> bool;

        #[cxx_name = "SetDev"]
        #[namespace = "netsim::config"]
        fn set_dev(flag: bool);

        // Ranging

        #[cxx_name = "DistanceToRssi"]
        fn distance_to_rssi(tx_power: i8, distance: f32) -> i8;

        // Version

        #[cxx_name = "GetVersion"]
        fn get_version() -> String;

        // Service

        type Service;
        #[cxx_name = "CreateService"]
        fn create_service() -> Box<Service>;
        #[cxx_name = "SetUp"]
        fn set_up(self: &Service);
        #[cxx_name = "Run"]
        fn run(self: &Service);

        // System

        #[cxx_name = "NetsimdTempDirString"]
        fn netsimd_temp_dir_string() -> String;

        // handlers for gRPC server's invocation of API calls

        #[cxx_name = "HandleCaptureCxx"]
        fn handle_capture_cxx(
            responder: Pin<&mut CxxServerResponseWriter>,
            method: String,
            param: String,
            body: String,
        );

        #[cxx_name = "HandleDeviceCxx"]
        fn handle_device_cxx(
            responder: Pin<&mut CxxServerResponseWriter>,
            method: String,
            param: String,
            body: String,
        );

        // Transport.

        #[cxx_name = HandleResponse]
        #[namespace = "netsim::transport"]
        fn handle_response(kind: u32, facade_id: u32, packet: &CxxVector<u8>, packet_type: u8);

        #[cxx_name = RegisterGrpcTransport]
        #[namespace = "netsim::transport"]
        fn register_grpc_transport(kind: u32, facade_id: u32);

        #[cxx_name = UnregisterGrpcTransport]
        #[namespace = "netsim::transport"]
        fn unregister_grpc_transport(kind: u32, facade_id: u32);

        // Device Resource
        type AddChipResultCxx;
        #[cxx_name = "GetDeviceId"]
        fn get_device_id(self: &AddChipResultCxx) -> u32;
        #[cxx_name = "GetChipId"]
        fn get_chip_id(self: &AddChipResultCxx) -> u32;
        #[cxx_name = "GetFacadeId"]
        fn get_facade_id(self: &AddChipResultCxx) -> u32;
        #[cxx_name = "IsError"]
        fn is_error(self: &AddChipResultCxx) -> bool;

        #[cxx_name = AddChipCxx]
        #[namespace = "netsim::device"]
        fn add_chip_cxx(
            device_guid: &str,
            device_name: &str,
            chip_kind: &CxxString,
            chip_name: &str,
            chip_manufacturer: &str,
            chip_product_name: &str,
        ) -> Box<AddChipResultCxx>;

        #[cxx_name = RemoveChipCxx]
        #[namespace = "netsim::device"]
        fn remove_chip_cxx(device_id: u32, chip_id: u32);

        #[cxx_name = GetDistanceCxx]
        #[namespace = "netsim::device"]
        fn get_distance_cxx(a: u32, b: u32) -> f32;

        #[cxx_name = IsShutdownTimeCxx]
        #[namespace = "netsim::device"]
        fn is_shutdown_time_cxx() -> bool;

        // Capture Resource

        #[cxx_name = UpdateCaptures]
        #[namespace = "netsim::capture"]
        fn update_captures();

        #[cxx_name = HandleRequest]
        #[namespace = "netsim::capture"]
        fn handle_packet_request(
            kind: u32,
            facade_id: u32,
            packet: &CxxVector<u8>,
            packet_type: u32,
        );

        #[cxx_name = HandleResponse]
        #[namespace = "netsim::capture"]
        fn handle_packet_response(
            kind: u32,
            facade_id: u32,
            packet: &CxxVector<u8>,
            packet_type: u32,
        );

        // Clearing out all pcap Files in temp directory

        #[cxx_name = ClearPcapFiles]
        #[namespace = "netsim::capture"]
        fn clear_pcap_files() -> bool;

        // Rust Bluetooth device.
        #[namespace = "netsim::hci::facade"]
        type DynRustBluetoothChipCallbacks;

        #[cxx_name = Tick]
        #[namespace = "netsim::hci::facade"]
        fn tick(dyn_callbacks: &mut DynRustBluetoothChipCallbacks);

        #[cxx_name = ReceiveLinkLayerPacket]
        #[namespace = "netsim::hci::facade"]
        fn receive_link_layer_packet(
            dyn_callbacks: &mut DynRustBluetoothChipCallbacks,
            source_address: String,
            destination_address: String,
            packet_type: u8,
            packet: &[u8],
        );

        // Bluetooth facade.
        #[namespace = "netsim::hci::facade"]
        type AddRustDeviceResult;
        #[cxx_name = "CreateAddRustDeviceResult"]
        #[namespace = "netsim::hci"]
        fn create_add_rust_device_result(
            facade_id: u32,
            rust_chip: UniquePtr<RustBluetoothChip>,
        ) -> Box<AddRustDeviceResult>;

        // Uwb Facade.

        #[cxx_name = HandleUwbRequestCxx]
        #[namespace = "netsim::uwb"]
        fn handle_uwb_request(facade_id: u32, packet: &[u8]);

        #[cxx_name = PatchCxx]
        #[namespace = "netsim::uwb::facade"]
        pub fn uwb_patch(_facade_id: u32, _proto_bytes: &[u8]);

        #[cxx_name = GetCxx]
        #[namespace = "netsim::uwb::facade"]
        pub fn uwb_get(_facade_id: u32) -> Vec<u8>;

        #[cxx_name = Reset]
        #[namespace = "netsim::uwb::facade"]
        pub fn uwb_reset(_facade_id: u32);

        #[cxx_name = Remove]
        #[namespace = "netsim::uwb::facade"]
        pub fn uwb_remove(_facade_id: u32);

        #[cxx_name = Add]
        #[namespace = "netsim::uwb::facade"]
        pub fn uwb_add(_chip_id: u32) -> u32;

        #[cxx_name = Start]
        #[namespace = "netsim::uwb::facade"]
        pub fn uwb_start();

        #[cxx_name = Stop]
        #[namespace = "netsim::uwb::facade"]
        pub fn uwb_stop();

    }

    unsafe extern "C++" {
        /// A C++ class which can be used to respond to a request.
        include!("frontend/server_response_writable.h");

        #[namespace = "netsim::frontend"]
        type CxxServerResponseWriter;

        #[namespace = "netsim::frontend"]
        fn put_ok_with_length(self: &CxxServerResponseWriter, mime_type: &CxxString, length: usize);

        #[namespace = "netsim::frontend"]
        fn put_chunk(self: &CxxServerResponseWriter, chunk: &[u8]);

        #[namespace = "netsim::frontend"]
        fn put_ok(self: &CxxServerResponseWriter, mime_type: &CxxString, body: &CxxString);

        #[namespace = "netsim::frontend"]
        fn put_error(self: &CxxServerResponseWriter, error_code: u32, error_message: &CxxString);

        include!("packet_hub/packet_hub.h");

        #[rust_name = "handle_request_cxx"]
        #[namespace = "netsim::packet_hub"]
        fn HandleRequestCxx(kind: u32, facade_id: u32, packet: &Vec<u8>, packet_type: u8);

        // Grpc server.
        include!("backend/backend_packet_hub.h");

        #[rust_name = handle_grpc_response]
        #[namespace = "netsim::backend"]
        fn HandleResponseCxx(kind: u32, facade_id: u32, packet: &CxxVector<u8>, packet_type: u8);

        // Bluetooth facade.
        include!("hci/hci_packet_hub.h");

        #[rust_name = handle_bt_request]
        #[namespace = "netsim::hci"]
        fn HandleBtRequestCxx(facade_id: u32, packet_type: u8, packet: &Vec<u8>);

        // Rust Bluetooth device.
        include!("hci/rust_device.h");

        #[namespace = "netsim::hci::facade"]
        type RustBluetoothChip;
        #[rust_name = send_link_layer_packet]
        #[namespace = "netsim::hci::facade"]
        fn SendLinkLayerPacket(
            self: &RustBluetoothChip,
            packet: &[u8],
            packet_type: u8,
            tx_power: i8,
        );

        #[rust_name = generate_advertising_packet]
        #[namespace = "netsim::hci::facade"]
        fn GenerateAdvertisingPacket(address: &String, packet: &[u8]) -> Vec<u8>;

        #[rust_name = generate_scan_response_packet]
        #[namespace = "netsim::hci::facade"]
        fn GenerateScanResponsePacket(
            source_address: &String,
            destination_address: &String,
            packet: &[u8],
        ) -> Vec<u8>;

        include!("hci/bluetooth_facade.h");

        #[rust_name = bluetooth_patch_cxx]
        #[namespace = "netsim::hci::facade"]
        pub fn PatchCxx(facade_id: u32, proto_bytes: &[u8]);

        #[rust_name = bluetooth_get_cxx]
        #[namespace = "netsim::hci::facade"]
        pub fn GetCxx(facade_id: u32) -> Vec<u8>;

        #[rust_name = bluetooth_reset]
        #[namespace = "netsim::hci::facade"]
        pub fn Reset(facade_id: u32);

        #[rust_name = bluetooth_remove]
        #[namespace = "netsim::hci::facade"]
        pub fn Remove(facade_id: u32);

        #[rust_name = bluetooth_add]
        #[namespace = "netsim::hci::facade"]
        pub fn Add(_chip_id: u32) -> u32;

        /*
        From https://cxx.rs/binding/box.html#restrictions,
        ```
        If T is an opaque Rust type, the Rust type is required to be Sized i.e. size known at compile time. In the future we may introduce support for dynamically sized opaque Rust types.
        ```

        The workaround is using Box<dyn MyData> (fat pointer) as the opaque type.
        Reference:
        - Passing trait objects to C++. https://github.com/dtolnay/cxx/issues/665.
        - Exposing trait methods to C++. https://github.com/dtolnay/cxx/issues/667
                */
        #[rust_name = bluetooth_add_rust_device]
        #[namespace = "netsim::hci::facade"]
        pub fn AddRustDevice(
            device_id: u32,
            callbacks: Box<DynRustBluetoothChipCallbacks>,
            string_type: &CxxString,
            address: &CxxString,
        ) -> Box<AddRustDeviceResult>;

        #[rust_name = bluetooth_start]
        #[namespace = "netsim::hci::facade"]
        pub fn Start();

        #[rust_name = bluetooth_stop]
        #[namespace = "netsim::hci::facade"]
        pub fn Stop();

        // WiFi facade.
        include!("wifi/wifi_packet_hub.h");

        #[rust_name = handle_wifi_request]
        #[namespace = "netsim::wifi"]
        fn HandleWifiRequestCxx(facade_id: u32, packet: &Vec<u8>);

        include!("wifi/wifi_facade.h");

        #[rust_name = wifi_patch_cxx]
        #[namespace = "netsim::wifi::facade"]
        pub fn PatchCxx(facade_id: u32, proto_bytes: &[u8]);

        #[rust_name = wifi_get_cxx]
        #[namespace = "netsim::wifi::facade"]
        pub fn GetCxx(facade_id: u32) -> Vec<u8>;

        #[rust_name = wifi_reset]
        #[namespace = "netsim::wifi::facade"]
        pub fn Reset(facade_id: u32);

        #[rust_name = wifi_remove]
        #[namespace = "netsim::wifi::facade"]
        pub fn Remove(facade_id: u32);

        #[rust_name = wifi_add]
        #[namespace = "netsim::wifi::facade"]
        pub fn Add(_chip_id: u32) -> u32;

        #[rust_name = wifi_start]
        #[namespace = "netsim::wifi::facade"]
        pub fn Start();

        #[rust_name = wifi_stop]
        #[namespace = "netsim::wifi::facade"]
        pub fn Stop();

    }
}

// It's required so `RustBluetoothChip` can be sent between threads safely.
//Ref: How to use opaque types in threads? https://github.com/dtolnay/cxx/issues/1175
unsafe impl Send for ffi::RustBluetoothChip {}

type DynRustBluetoothChipCallbacks = Box<dyn RustBluetoothChipCallbacks>;

fn tick(dyn_callbacks: &mut DynRustBluetoothChipCallbacks) {
    (**dyn_callbacks).tick();
}

fn receive_link_layer_packet(
    dyn_callbacks: &mut DynRustBluetoothChipCallbacks,
    source_address: String,
    destination_address: String,
    packet_type: u8,
    packet: &[u8],
) {
    (**dyn_callbacks).receive_link_layer_packet(
        source_address,
        destination_address,
        packet_type,
        packet,
    );
}

/// CxxServerResponseWriter is defined in server_response_writable.h
/// Wrapper struct allows the impl to discover the respective C++ methods
struct CxxServerResponseWriterWrapper<'a> {
    writer: Pin<&'a mut CxxServerResponseWriter>,
}

impl ServerResponseWritable for CxxServerResponseWriterWrapper<'_> {
    fn put_ok_with_length(&mut self, mime_type: &str, length: usize, _headers: StrHeaders) {
        let_cxx_string!(mime_type = mime_type);
        self.writer.put_ok_with_length(&mime_type, length);
    }
    fn put_chunk(&mut self, chunk: &[u8]) {
        self.writer.put_chunk(chunk);
    }
    fn put_ok(&mut self, mime_type: &str, body: &str, _headers: StrHeaders) {
        let_cxx_string!(mime_type = mime_type);
        let_cxx_string!(body = body);
        self.writer.put_ok(&mime_type, &body);
    }
    fn put_error(&mut self, error_code: u16, error_message: &str) {
        let_cxx_string!(error_message = error_message);
        self.writer.put_error(error_code.into(), &error_message);
    }

    fn put_ok_with_vec(&mut self, _mime_type: &str, _body: Vec<u8>, _headers: StrHeaders) {
        todo!()
    }
    fn put_ok_switch_protocol(&mut self, _connection: &str) {
        todo!()
    }
}
