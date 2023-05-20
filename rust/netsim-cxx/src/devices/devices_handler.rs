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

// devices_handler.rs
//
// Provides the API for the frontend and backend to interact with devices.
//
// The Devices struct is a singleton for the devices collection.
//
// Additional functions are
// -- inactivity instant
// -- vending device identifiers

use super::chip::ChipIdentifier;
use super::device::DeviceIdentifier;
use super::id_factory::IdFactory;
use crate::devices::device::AddChipResult;
use crate::devices::device::Device;
use crate::ffi::CxxServerResponseWriter;
use crate::http_server::http_request::HttpHeaders;
use crate::http_server::http_request::HttpRequest;
use crate::http_server::server_response::ResponseWritable;
use crate::CxxServerResponseWriterWrapper;
use cxx::CxxString;
use cxx::UniquePtr;
use frontend_proto::common::ChipKind as ProtoChipKind;
use frontend_proto::frontend::ListDeviceResponse;
use frontend_proto::frontend::PatchDeviceRequest;
use frontend_proto::model::Position as ProtoPosition;
use frontend_proto::model::Scene as ProtoScene;
use lazy_static::lazy_static;
use log::{error, info};
use protobuf_json_mapping::merge_from_str;
use protobuf_json_mapping::print_to_string_with_options;
use protobuf_json_mapping::PrintOptions;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::RwLock;
use std::sync::RwLockWriteGuard;
use std::time::Duration;
use std::time::Instant;

const INITIAL_DEVICE_ID: DeviceIdentifier = 0;
const JSON_PRINT_OPTION: PrintOptions = PrintOptions {
    enum_values_int: false,
    proto_field_name: false,
    always_output_default_values: true,
    _future_options: (),
};

lazy_static! {
    static ref DEVICES: RwLock<Devices> = RwLock::new(Devices::new());
}
static IDLE_SECS_FOR_SHUTDOWN: u64 = 120;

/// The Device resource is a singleton that manages all devices.
struct Devices {
    // BTreeMap allows ListDevice to output devices in order of identifiers.
    devices: BTreeMap<DeviceIdentifier, Device>,
    id_factory: IdFactory<DeviceIdentifier>,
    pub idle_since: Option<Instant>,
}

impl Devices {
    fn new() -> Self {
        Devices {
            devices: BTreeMap::new(),
            id_factory: IdFactory::new(INITIAL_DEVICE_ID, 1),
            idle_since: None,
        }
    }
}

#[allow(dead_code)]
fn notify_all() {
    // TODO
}

/// Returns a Result<AddChipResult, String> after adding chip to resource.
/// add_chip is called by the transport layer when a new chip is attached.
///
/// The guid is a transport layer identifier for the device (host:port)
/// that is adding the chip.
#[allow(dead_code)]
fn add_chip(
    device_guid: &str,
    device_name: &str,
    chip_kind: ProtoChipKind,
    chip_name: &str,
    chip_manufacturer: &str,
    chip_product_name: &str,
) -> Result<AddChipResult, String> {
    let mut resource = DEVICES.write().unwrap();
    resource.idle_since = None;
    let device_id = get_or_create_device(&mut resource, device_guid, device_name);
    // This is infrequent, so we can afford to do another lookup for the device.
    resource.devices.get_mut(&device_id).unwrap().add_chip(
        device_name,
        chip_kind,
        chip_name,
        chip_manufacturer,
        chip_product_name,
    )
}

/// An AddChip function for Rust Device API.
/// The backend gRPC code will be invoking this method.
pub fn add_chip_rust(
    device_guid: &str,
    device_name: &str,
    chip_kind: &CxxString,
    chip_name: &str,
    chip_manufacturer: &str,
    chip_product_name: &str,
) -> UniquePtr<crate::ffi::AddChipResult> {
    let chip_kind_proto = match chip_kind.to_string().as_str() {
        "BLUETOOTH" => ProtoChipKind::BLUETOOTH,
        "WIFI" => ProtoChipKind::WIFI,
        "UWB" => ProtoChipKind::UWB,
        _ => ProtoChipKind::UNSPECIFIED,
    };
    match add_chip(
        device_guid,
        device_name,
        chip_kind_proto,
        chip_name,
        chip_manufacturer,
        chip_product_name,
    ) {
        Ok(result) => {
            info!("Rust Device API Add Chip Success");
            crate::ffi::new_add_chip_result(
                result.device_id as u32,
                result.chip_id as u32,
                result.facade_id,
            )
        }
        Err(err) => {
            error!("Rust Device API Add Chip Error: {err}");
            crate::ffi::new_add_chip_result(u32::MAX, u32::MAX, u32::MAX)
        }
    }
}

/// Get or create a device.
fn get_or_create_device(
    resource: &mut RwLockWriteGuard<Devices>,
    guid: &str,
    name: &str,
) -> DeviceIdentifier {
    // Check if a device with the given guid already exists
    if let Some(existing_device) = resource.devices.values().find(|d| d.guid == guid) {
        // A device with the same guid already exists, return it
        existing_device.id
    } else {
        // No device with the same guid exists, insert the new device
        let new_id = resource.id_factory.next_id();
        resource.devices.insert(new_id, Device::new(new_id, guid.to_string(), name.to_string()));
        new_id
    }
}

/// Remove a device from the simulation.
///
/// Called when the last chip for the device is removed.
fn remove_device(
    resource: &mut RwLockWriteGuard<Devices>,
    id: DeviceIdentifier,
) -> Result<(), String> {
    resource.devices.remove(&id).ok_or(format!("Error removing device id {id}"))?;
    if resource.devices.is_empty() {
        resource.idle_since = Some(Instant::now());
    }
    Ok(())
}

/// Remove a chip from a device.
///
/// Called when the packet transport for the chip shuts down.
#[allow(dead_code)]
fn remove_chip(device_id: DeviceIdentifier, chip_id: ChipIdentifier) -> Result<(), String> {
    let mut resource = DEVICES.write().unwrap();
    let is_empty = match resource.devices.entry(device_id) {
        Entry::Occupied(mut entry) => {
            let device = entry.get_mut();
            device.remove_chip(chip_id)?;
            device.chips.is_empty()
        }
        Entry::Vacant(_) => return Err(format!("RemoveChip device id {device_id} not found")),
    };
    if is_empty {
        remove_device(&mut resource, device_id)?;
    }
    Ok(())
}

/// A RemoveChip function for Rust Device API.
/// The backend gRPC code will be invoking this method.
pub fn remove_chip_rust(device_id: u32, chip_id: u32) {
    match remove_chip(device_id as i32, chip_id as i32) {
        Ok(_) => info!("Rust Device API Remove Chip Success"),
        Err(err) => error!("Rust Device API Remove Chip Failure: {err}"),
    }
}

// lock the devices, find the id and call the patch function
#[allow(dead_code)]
fn patch_device(id_option: Option<DeviceIdentifier>, patch_json: &str) -> Result<(), String> {
    let mut patch_device_request = PatchDeviceRequest::new();
    if merge_from_str(&mut patch_device_request, patch_json).is_ok() {
        let mut resource = DEVICES.write().unwrap();
        let proto_device = patch_device_request.device;
        match id_option {
            Some(id) => match resource.devices.get_mut(&id) {
                Some(device) => device.patch(&proto_device),
                None => Err(format!("No such device with id {id}")),
            },
            None => {
                let mut multiple_matches = false;
                let mut target: Option<&mut Device> = None;
                for device in resource.devices.values_mut() {
                    if device.name.contains(&proto_device.name) {
                        if device.name == proto_device.name {
                            return device.patch(&proto_device);
                        }
                        multiple_matches = target.is_some();
                        target = Some(device);
                    }
                }
                if multiple_matches {
                    return Err(format!(
                        "Multiple ambiguous matches were found with substring {}",
                        proto_device.name
                    ));
                }
                match target {
                    Some(device) => device.patch(&proto_device),
                    None => Err(format!("No such device with name {}", proto_device.name)),
                }
            }
        }
    } else {
        Err(format!("Incorrect format of patch json {}", patch_json))
    }
}

fn distance(a: &ProtoPosition, b: &ProtoPosition) -> f32 {
    ((b.x - a.x).powf(2.0) + (b.y - a.y).powf(2.0) + (b.z - a.z).powf(2.0)).sqrt()
}

#[allow(dead_code)]
fn get_distance(id: DeviceIdentifier, other_id: DeviceIdentifier) -> Result<f32, String> {
    print!("get_distance({:?}, {:?}) = ", id, other_id);
    let devices = &DEVICES.read().unwrap().devices;
    let a = devices
        .get(&id)
        .map(|device_ref| device_ref.position.clone())
        .ok_or(format!("No such device with id {id}"))?;
    let b = devices
        .get(&other_id)
        .map(|device_ref| device_ref.position.clone())
        .ok_or(format!("No such device with id {other_id}"))?;
    Ok(distance(&a, &b))
}

/// A GetDistance function for Rust Device API.
/// The backend gRPC code will be invoking this method.
pub fn get_distance_rust(a: u32, b: u32) -> f32 {
    match get_distance(a as i32, b as i32) {
        Ok(distance) => distance,
        Err(err) => {
            error!("Rust Device API Get Distance Error: {err}");
            0.0
        }
    }
}

#[allow(dead_code)]
fn get_devices() -> Result<ProtoScene, String> {
    let mut scene = ProtoScene::new();
    // iterate over the devices and add each to the scene
    let resource = DEVICES.read().unwrap();
    for device in resource.devices.values() {
        scene.devices.push(device.get()?);
    }
    Ok(scene)
}

#[allow(dead_code)]
fn reset(id: DeviceIdentifier) -> Result<(), String> {
    let mut resource = DEVICES.write().unwrap();
    match resource.devices.get_mut(&id) {
        Some(device) => device.reset(),
        None => Err(format!("No such device with id {id}")),
    }
}

#[allow(dead_code)]
fn reset_all() -> Result<(), String> {
    let mut resource = DEVICES.write().unwrap();
    for device in resource.devices.values_mut() {
        device.reset()?;
    }
    Ok(())
}

#[allow(dead_code)]
fn get_secs_until_idle_shutdown() -> Option<u32> {
    if let Some(idle_since) = DEVICES.read().unwrap().idle_since {
        let remaining_secs = idle_since
            .elapsed()
            .saturating_sub(Duration::from_secs(IDLE_SECS_FOR_SHUTDOWN))
            .as_secs();
        Some(remaining_secs.try_into().unwrap())
    } else {
        None
    }
}

/// Performs PatchDevice to patch a single device
fn handle_device_patch(writer: ResponseWritable, id: Option<DeviceIdentifier>, patch_json: &str) {
    match patch_device(id, patch_json) {
        Ok(()) => writer.put_ok("text/plain", "Device Patch Success", &[]),
        Err(err) => writer.put_error(404, err.as_str()),
    }
}

/// Performs ListDevices to get the list of Devices and write to writer.
fn handle_device_list(writer: ResponseWritable) {
    let devices = get_devices().unwrap();
    // Instantiate ListDeviceResponse and add Devices
    let mut response = ListDeviceResponse::new();
    for device in devices.devices {
        response.devices.push(device);
    }

    // Perform protobuf-json-mapping with the given protobuf
    if let Ok(json_response) = print_to_string_with_options(&response, &JSON_PRINT_OPTION) {
        writer.put_ok("text/json", &json_response, &[])
    } else {
        writer.put_error(404, "proto to JSON mapping failure")
    }
}

/// Performs ResetDevice for all devices
fn handle_device_reset(writer: ResponseWritable) {
    match reset_all() {
        Ok(()) => writer.put_ok("text/plain", "Device Reset Success", &[]),
        Err(err) => writer.put_error(404, err.as_str()),
    }
}

/// The Rust device handler used directly by Http frontend or handle_device_cxx for LIST, GET, and PATCH
pub fn handle_device(request: &HttpRequest, param: &str, writer: ResponseWritable) {
    // Route handling
    if request.uri.as_str() == "/v1/devices" {
        // Routes with ID not specified
        match request.method.as_str() {
            "GET" => {
                handle_device_list(writer);
            }
            "PUT" => {
                handle_device_reset(writer);
            }
            "PATCH" => {
                let body = &request.body;
                let patch_json = String::from_utf8(body.to_vec()).unwrap();
                handle_device_patch(writer, None, patch_json.as_str());
            }
            _ => writer.put_error(404, "Not found."),
        }
    } else {
        // Routes with ID specified
        match request.method.as_str() {
            "PATCH" => {
                let id = match param.parse::<i32>() {
                    Ok(num) => num,
                    Err(_) => {
                        writer.put_error(404, "Incorrect Id type for devices, ID should be i32.");
                        return;
                    }
                };
                let body = &request.body;
                let patch_json = String::from_utf8(body.to_vec()).unwrap();
                handle_device_patch(writer, Some(id), patch_json.as_str());
            }
            _ => writer.put_error(404, "Not found."),
        }
    }
}

/// Device handler cxx for grpc server to call
pub fn handle_device_cxx(
    responder: Pin<&mut CxxServerResponseWriter>,
    method: String,
    param: String,
    body: String,
) {
    let mut request = HttpRequest {
        method,
        uri: String::new(),
        headers: HttpHeaders::new(),
        version: "1.1".to_string(),
        body: body.as_bytes().to_vec(),
    };
    if param.is_empty() {
        request.uri = "/v1/devices".to_string();
    } else {
        request.uri = format!("/v1/devices/{}", param)
    }
    handle_device(
        &request,
        param.as_str(),
        &mut CxxServerResponseWriterWrapper { writer: responder },
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, Once};

    use frontend_proto::model::{Device as ProtoDevice, Orientation as ProtoOrientation, State};
    use netsim_common::util::netsim_logger::init_for_test;
    use protobuf_json_mapping::print_to_string;

    use super::*;

    // Since rust unit tests occur in parallel. We must lock each test case
    // to avoid unwanted interleaving operations on DEVICES
    lazy_static! {
        static ref MUTEX: Mutex<()> = Mutex::new(());
    }

    // This allows Log init method to be invoked once when running all tests.
    static INIT: Once = Once::new();

    /// Logger setup function that is only run once, even if called multiple times.
    fn logger_setup() {
        INIT.call_once(|| {
            init_for_test();
        });
    }

    /// TestChipParameters struct to invoke add_chip
    /// This struct contains parameters required to invoke add_chip.
    /// This will eventually be invoked by the facades.
    struct TestChipParameters<'a> {
        device_guid: &'a str,
        device_name: &'a str,
        chip_kind: ProtoChipKind,
        chip_name: &'a str,
        chip_manufacturer: &'a str,
        chip_product_name: &'a str,
    }

    impl TestChipParameters<'_> {
        fn add_chip(&self) -> Result<AddChipResult, String> {
            super::add_chip(
                self.device_guid,
                self.device_name,
                self.chip_kind,
                self.chip_name,
                self.chip_manufacturer,
                self.chip_product_name,
            )
        }

        fn get_or_create_device(&self) -> DeviceIdentifier {
            let mut resource = DEVICES.write().unwrap();
            super::get_or_create_device(&mut resource, self.device_guid, self.device_name)
        }
    }

    /// helper function for test cases to instantiate ProtoPosition
    fn new_position(x: f32, y: f32, z: f32) -> ProtoPosition {
        ProtoPosition { x, y, z, ..Default::default() }
    }

    fn new_orientation(yaw: f32, pitch: f32, roll: f32) -> ProtoOrientation {
        ProtoOrientation { yaw, pitch, roll, ..Default::default() }
    }

    /// helper function for test cases to refresh DEVICES
    fn refresh_resource() {
        let mut resource = DEVICES.write().unwrap();
        resource.devices = BTreeMap::new();
        resource.id_factory = IdFactory::new(1000, 1);
        resource.idle_since = None
    }

    fn test_chip_1_bt() -> TestChipParameters<'static> {
        TestChipParameters {
            device_guid: "guid-fs-1",
            device_name: "test-device-name-1",
            chip_kind: ProtoChipKind::BLUETOOTH,
            chip_name: "bt_chip_name",
            chip_manufacturer: "netsim",
            chip_product_name: "netsim_bt",
        }
    }

    fn test_chip_1_wifi() -> TestChipParameters<'static> {
        TestChipParameters {
            device_guid: "guid-fs-1",
            device_name: "test-device-name-1",
            chip_kind: ProtoChipKind::WIFI,
            chip_name: "bt_chip_name",
            chip_manufacturer: "netsim",
            chip_product_name: "netsim_bt",
        }
    }

    fn test_chip_2_bt() -> TestChipParameters<'static> {
        TestChipParameters {
            device_guid: "guid-fs-2",
            device_name: "test-device-name-2",
            chip_kind: ProtoChipKind::BLUETOOTH,
            chip_name: "bt_chip_name",
            chip_manufacturer: "netsim",
            chip_product_name: "netsim_bt",
        }
    }

    #[test]
    fn test_distance() {
        // Pythagorean quadruples
        let a = new_position(0.0, 0.0, 0.0);
        let mut b = new_position(1.0, 2.0, 2.0);
        assert_eq!(distance(&a, &b), 3.0);
        b = new_position(2.0, 3.0, 6.0);
        assert_eq!(distance(&a, &b), 7.0);
    }

    #[test]
    fn test_add_chip() {
        // Avoiding Interleaving Operations
        let _lock = MUTEX.lock().unwrap();

        // Initializing Logger
        logger_setup();

        // Adding a chip
        refresh_resource();
        let chip_params = test_chip_1_bt();
        let chip_result = chip_params.add_chip().unwrap();
        match get_devices().unwrap().devices.get(0) {
            Some(device) => {
                let chip = device.chips.get(0).unwrap();
                assert_eq!(chip_params.chip_kind, chip.kind.enum_value_or_default());
                assert_eq!(chip_params.chip_manufacturer, chip.manufacturer);
                assert_eq!(chip_params.chip_name, chip.name);
                assert_eq!(chip_params.chip_product_name, chip.product_name);
                assert_eq!(chip_params.device_name, device.name);
            }
            None => unreachable!(),
        }
        let chip_id = chip_result.chip_id;

        // Adding duplicate chip
        let chip_result = chip_params.add_chip();
        assert!(chip_result.is_err());
        assert_eq!(
            chip_result.unwrap_err(),
            format!("Device::AddChip - duplicate at id {chip_id}, skipping.")
        );
    }

    #[test]
    fn test_get_or_create_device() {
        // Avoiding Interleaving Operations
        let _lock = MUTEX.lock().unwrap();

        // Initializing Logger
        logger_setup();

        // Creating a device and getting device
        refresh_resource();
        let bt_chip_params = test_chip_1_bt();
        let device_id = bt_chip_params.get_or_create_device();
        assert_eq!(device_id, 1000);
        let wifi_chip_params = test_chip_1_wifi();
        let device_id = wifi_chip_params.get_or_create_device();
        assert_eq!(device_id, 1000);
    }

    #[test]
    fn test_patch_device() {
        // Avoiding Interleaving Operations
        let _lock = MUTEX.lock().unwrap();

        // Initializing Logger
        logger_setup();

        // Patching device position and orientation by id
        refresh_resource();
        let chip_params = test_chip_1_bt();
        let chip_result = chip_params.add_chip().unwrap();
        let mut patch_device_request = PatchDeviceRequest::new();
        let mut proto_device = ProtoDevice::new();
        let request_position = new_position(1.1, 2.2, 3.3);
        let request_orientation = new_orientation(4.4, 5.5, 6.6);
        proto_device.name = chip_params.device_name.into();
        proto_device.visible = State::OFF.into();
        proto_device.position = Some(request_position.clone()).into();
        proto_device.orientation = Some(request_orientation.clone()).into();
        patch_device_request.device = Some(proto_device.clone()).into();
        let patch_json = print_to_string(&patch_device_request).unwrap();
        patch_device(Some(chip_result.device_id), patch_json.as_str()).unwrap();
        match get_devices().unwrap().devices.get(0) {
            Some(device) => {
                assert_eq!(device.position.x, request_position.x);
                assert_eq!(device.position.y, request_position.y);
                assert_eq!(device.position.z, request_position.z);
                assert_eq!(device.orientation.yaw, request_orientation.yaw);
                assert_eq!(device.orientation.pitch, request_orientation.pitch);
                assert_eq!(device.orientation.roll, request_orientation.roll);
                assert_eq!(device.visible.enum_value_or_default(), State::OFF);
            }
            None => unreachable!(),
        }

        // Patch device by name with substring match
        proto_device.name = "test".into();
        patch_device_request.device = Some(proto_device).into();
        let patch_json = print_to_string(&patch_device_request).unwrap();
        assert!(patch_device(None, patch_json.as_str()).is_ok());
    }

    #[test]
    fn test_patch_error() {
        // Avoiding Interleaving Operations
        let _lock = MUTEX.lock().unwrap();

        // Initializing Logger
        logger_setup();

        // Patch Error Testing
        refresh_resource();
        let bt_chip_params = test_chip_1_bt();
        let bt_chip2_params = test_chip_2_bt();
        let bt_chip_result = bt_chip_params.add_chip().unwrap();
        bt_chip2_params.add_chip().unwrap();

        // Incorrect value type
        let error_json = r#"{"device": {"name": "test-device-name-1", "position": 1.1}}"#;
        let patch_result = patch_device(Some(bt_chip_result.device_id), error_json);
        assert!(patch_result.is_err());
        assert_eq!(
            patch_result.unwrap_err(),
            format!("Incorrect format of patch json {}", error_json)
        );

        // Incorrect key
        let error_json = r#"{"device": {"name": "test-device-name-1", "hello": "world"}}"#;
        let patch_result = patch_device(Some(bt_chip_result.device_id), error_json);
        assert!(patch_result.is_err());
        assert_eq!(
            patch_result.unwrap_err(),
            format!("Incorrect format of patch json {}", error_json)
        );

        // Incorrect Id
        let error_json = r#"{"device": {"name": "test-device-name-1"}}"#;
        let patch_result = patch_device(Some(INITIAL_DEVICE_ID - 1), error_json);
        assert!(patch_result.is_err());
        assert_eq!(
            patch_result.unwrap_err(),
            format!("No such device with id {}", INITIAL_DEVICE_ID - 1)
        );

        // Incorrect name
        let error_json = r#"{"device": {"name": "wrong-name"}}"#;
        let patch_result = patch_device(None, error_json);
        assert!(patch_result.is_err());
        assert_eq!(patch_result.unwrap_err(), "No such device with name wrong-name");

        // Multiple ambiguous matching
        let error_json = r#"{"device": {"name": "test-device"}}"#;
        let patch_result = patch_device(None, error_json);
        assert!(patch_result.is_err());
        assert_eq!(
            patch_result.unwrap_err(),
            "Multiple ambiguous matches were found with substring test-device"
        );
    }

    #[test]
    fn test_adding_two_chips() {
        // Avoiding Interleaving Operations
        let _lock = MUTEX.lock().unwrap();

        // Initializing Logger
        logger_setup();

        // Adding two chips of the same device
        refresh_resource();
        let bt_chip_params = test_chip_1_bt();
        let wifi_chip_params = test_chip_1_wifi();
        let bt_chip_result = bt_chip_params.add_chip().unwrap();
        let wifi_chip_result = wifi_chip_params.add_chip().unwrap();
        assert_eq!(bt_chip_result.device_id, wifi_chip_result.device_id);
        assert_eq!(get_devices().unwrap().devices.len(), 1);
        let scene = get_devices().unwrap();
        let device = scene.devices.get(0).unwrap();
        assert_eq!(device.id, bt_chip_result.device_id);
        assert_eq!(device.name, bt_chip_params.device_name);
        assert_eq!(device.visible.enum_value_or_default(), State::ON);
        assert!(device.position.is_some());
        assert!(device.orientation.is_some());
        assert_eq!(device.chips.len(), 2);
        for chip in &device.chips {
            assert!(chip.id == bt_chip_result.chip_id || chip.id == wifi_chip_result.chip_id);
            if chip.id == bt_chip_result.chip_id {
                assert!(chip.has_bt());
            } else if chip.id == wifi_chip_result.chip_id {
                assert!(chip.has_wifi());
            } else {
                unreachable!();
            }
        }
    }

    #[test]
    fn test_reset() {
        // Avoiding Interleaving Operations
        let _lock = MUTEX.lock().unwrap();

        // Initializing Logger
        logger_setup();

        // Patching Device and Resetting scene
        refresh_resource();
        let chip_params = test_chip_1_bt();
        let chip_result = chip_params.add_chip().unwrap();
        let mut patch_device_request = PatchDeviceRequest::new();
        let mut proto_device = ProtoDevice::new();
        let request_position = new_position(10.0, 20.0, 30.0);
        let request_orientation = new_orientation(1.0, 2.0, 3.0);
        proto_device.name = chip_params.device_name.into();
        proto_device.visible = State::OFF.into();
        proto_device.position = Some(request_position).into();
        proto_device.orientation = Some(request_orientation).into();
        patch_device_request.device = Some(proto_device).into();
        patch_device(
            Some(chip_result.device_id),
            print_to_string(&patch_device_request).unwrap().as_str(),
        )
        .unwrap();
        match get_devices().unwrap().devices.get(0) {
            Some(device) => {
                assert_eq!(device.position.x, 10.0);
                assert_eq!(device.orientation.yaw, 1.0);
                assert_eq!(device.visible.enum_value_or_default(), State::OFF);
            }
            None => unreachable!(),
        }
        reset(chip_result.device_id).unwrap();
        match get_devices().unwrap().devices.get(0) {
            Some(device) => {
                assert_eq!(device.position.x, 0.0);
                assert_eq!(device.position.y, 0.0);
                assert_eq!(device.position.z, 0.0);
                assert_eq!(device.orientation.yaw, 0.0);
                assert_eq!(device.orientation.pitch, 0.0);
                assert_eq!(device.orientation.roll, 0.0);
                assert_eq!(device.visible.enum_value_or_default(), State::ON);
            }
            None => unreachable!(),
        }
    }

    #[test]
    fn test_remove_chip() {
        // Avoiding Interleaving Operations
        let _lock = MUTEX.lock().unwrap();

        // Initializing Logger
        logger_setup();

        // Add 2 chips of same device and 1 chip of different device
        refresh_resource();
        let bt_chip_params = test_chip_1_bt();
        let wifi_chip_params = test_chip_1_wifi();
        let bt_chip_2_params = test_chip_2_bt();
        let bt_chip_result = bt_chip_params.add_chip().unwrap();
        let wifi_chip_result = wifi_chip_params.add_chip().unwrap();
        let bt_chip_2_result = bt_chip_2_params.add_chip().unwrap();

        // Remove a bt chip of first device
        remove_chip(bt_chip_result.device_id, bt_chip_result.chip_id).unwrap();
        assert_eq!(get_devices().unwrap().devices.len(), 2);
        for device in get_devices().unwrap().devices {
            if device.id == wifi_chip_result.device_id {
                assert_eq!(wifi_chip_params.device_name, device.name);
            } else if device.id == bt_chip_2_result.device_id {
                assert_eq!(bt_chip_2_params.device_name, device.name);
            } else {
                unreachable!();
            }
        }

        // Remove a wifi chip of first device
        remove_chip(wifi_chip_result.device_id, wifi_chip_result.chip_id).unwrap();
        assert_eq!(get_devices().unwrap().devices.len(), 1);
        match get_devices().unwrap().devices.get(0) {
            Some(device) => assert_eq!(bt_chip_2_params.device_name, device.name),
            None => unreachable!(),
        }

        // Remove a bt chip of second device
        remove_chip(bt_chip_2_result.device_id, bt_chip_2_result.chip_id).unwrap();
        assert!(get_devices().unwrap().devices.is_empty());
    }

    #[test]
    fn test_remove_chip_error() {
        // Avoiding Interleaving Operations
        let _lock = MUTEX.lock().unwrap();

        // Initializing Logger
        logger_setup();

        // Add 2 chips of same device and 1 chip of different device
        refresh_resource();
        let bt_chip_params = test_chip_1_bt();
        let bt_chip_result = bt_chip_params.add_chip().unwrap();

        // Invoke remove_chip with incorrect chip_id.
        match remove_chip(bt_chip_result.device_id, 4000) {
            Ok(_) => unreachable!(),
            Err(err) => assert_eq!(err, "RemoveChip chip id 4000 not found"),
        }

        // Invoke remove_chip with incorrect device_id
        match remove_chip(4000, bt_chip_result.chip_id) {
            Ok(_) => unreachable!(),
            Err(err) => assert_eq!(err, "RemoveChip device id 4000 not found"),
        }
        assert_eq!(get_devices().unwrap().devices.len(), 1);
    }
}
