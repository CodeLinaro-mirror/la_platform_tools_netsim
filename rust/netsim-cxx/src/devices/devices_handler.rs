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
use super::chip::FacadeIdentifier;
use super::device::DeviceIdentifier;
use super::id_factory::IdFactory;
use crate::bluetooth as bluetooth_facade;
use crate::captures::handlers::update_captures;
use crate::devices::device::AddChipResult;
use crate::devices::device::Device;
use crate::events::Event;
use crate::ffi::CxxServerResponseWriter;
use crate::http_server::http_request::HttpHeaders;
use crate::http_server::http_request::HttpRequest;
use crate::http_server::server_response::ResponseWritable;
use crate::resource;
use crate::resource::clone_devices;
use crate::wifi as wifi_facade;
use crate::CxxServerResponseWriterWrapper;
use cxx::CxxString;
use frontend_proto::common::ChipKind as ProtoChipKind;
use frontend_proto::frontend::ListDeviceResponse;
use frontend_proto::frontend::PatchDeviceRequest;
use frontend_proto::model::Position as ProtoPosition;
use frontend_proto::model::Scene as ProtoScene;
use log::{error, info};
use protobuf_json_mapping::merge_from_str;
use protobuf_json_mapping::print_to_string_with_options;
use protobuf_json_mapping::PrintOptions;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::RwLockWriteGuard;
use std::time::Instant;

const INITIAL_DEVICE_ID: DeviceIdentifier = 1;
const JSON_PRINT_OPTION: PrintOptions = PrintOptions {
    enum_values_int: false,
    proto_field_name: false,
    always_output_default_values: true,
    _future_options: (),
};

static IDLE_SECS_FOR_SHUTDOWN: u64 = 15;

/// The Device resource is a singleton that manages all devices.
pub struct Devices {
    // BTreeMap allows ListDevice to output devices in order of identifiers.
    entries: BTreeMap<DeviceIdentifier, Device>,
    id_factory: IdFactory<DeviceIdentifier>,
    pub idle_since: Option<Instant>,
}

impl Devices {
    pub fn new() -> Self {
        Devices {
            entries: BTreeMap::new(),
            id_factory: IdFactory::new(INITIAL_DEVICE_ID, 1),
            idle_since: Some(Instant::now()),
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
pub fn add_chip(
    device_guid: &str,
    device_name: &str,
    chip_kind: ProtoChipKind,
    chip_name: &str,
    chip_manufacturer: &str,
    chip_product_name: &str,
) -> Result<AddChipResult, String> {
    let result = {
        let devices_arc = clone_devices();
        let mut devices = devices_arc.write().unwrap();
        devices.idle_since = None;
        let device_id = get_or_create_device(&mut devices, device_guid, device_name);
        // This is infrequent, so we can afford to do another lookup for the device.
        devices
            .entries
            .get_mut(&device_id)
            .ok_or(format!("Device not found for device_id: {device_id}"))?
            .add_chip(device_name, chip_kind, chip_name, chip_manufacturer, chip_product_name)
    };

    // Device resource is no longer locked
    match result {
        // id_tuple = (DeviceIdentifier, ChipIdentifier)
        Ok((device_id, chip_id)) => {
            let facade_id = match chip_kind {
                ProtoChipKind::BLUETOOTH => bluetooth_facade::bluetooth_add(device_id as u32),
                ProtoChipKind::BLUETOOTH_BEACON => bluetooth_facade::beacon::bluetooth_beacon_add(
                    device_id as u32,
                    chip_id as u32,
                    "beacon".to_string(),
                    chip_name.to_string(),
                ),
                ProtoChipKind::WIFI => wifi_facade::wifi_add(device_id as u32),
                _ => return Err(format!("Unknown chip kind: {:?}", chip_kind)),
            };
            // Add the facade_id into the resources
            {
                clone_devices()
                    .write()
                    .unwrap()
                    .entries
                    .get_mut(&device_id)
                    .ok_or(format!("Device not found for device_id: {device_id}"))?
                    .chips
                    .get_mut(&chip_id)
                    .ok_or(format!("Chip not found for device_id: {device_id}, chip_id:{chip_id}"))?
                    .facade_id = Some(facade_id);
            }
            info!(
                "Added Chip: device_name: {device_name}, chip_kind: {chip_kind:?}, device_id: {device_id}, chip_id: {chip_id}, facade_id: {facade_id}",
            );
            // Update Capture resource
            update_captures();
            resource::clone_events().lock().unwrap().publish(Event::ChipAdded {
                id: chip_id as u32,
                device_id: device_id as u32,
                facade_id,
                device_name: device_name.to_string(),
                kind: chip_kind,
            });
            Ok(AddChipResult { device_id, chip_id, facade_id })
        }
        Err(err) => {
            error!(
                "Failed to add chip: device_name: {device_name}, chip_kind: {chip_kind:?}, error: {err}",
            );
            Err(err)
        }
    }
}

/// AddChipResult for C++ to handle
pub struct AddChipResultCxx {
    device_id: u32,
    chip_id: u32,
    facade_id: u32,
    is_error: bool,
}

impl AddChipResultCxx {
    fn new(device_id: u32, chip_id: u32, facade_id: u32, is_error: bool) -> AddChipResultCxx {
        AddChipResultCxx { device_id, chip_id, facade_id, is_error }
    }

    pub fn get_device_id(&self) -> u32 {
        self.device_id
    }

    pub fn get_chip_id(&self) -> u32 {
        self.chip_id
    }

    pub fn get_facade_id(&self) -> u32 {
        self.facade_id
    }

    pub fn is_error(&self) -> bool {
        self.is_error
    }
}

/// An AddChip function for Rust Device API.
/// The backend gRPC code will be invoking this method.
pub fn add_chip_cxx(
    device_guid: &str,
    device_name: &str,
    chip_kind: &CxxString,
    chip_name: &str,
    chip_manufacturer: &str,
    chip_product_name: &str,
) -> Box<AddChipResultCxx> {
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
        Ok(result) => Box::new(AddChipResultCxx {
            device_id: result.device_id as u32,
            chip_id: result.chip_id as u32,
            facade_id: result.facade_id,
            is_error: false,
        }),
        Err(_) => Box::new(AddChipResultCxx {
            device_id: u32::MAX,
            chip_id: u32::MAX,
            facade_id: u32::MAX,
            is_error: true,
        }),
    }
}

/// Get or create a device.
fn get_or_create_device(devices: &mut Devices, guid: &str, name: &str) -> DeviceIdentifier {
    // Check if a device with the given guid already exists
    if let Some(existing_device) = devices.entries.values().find(|d| d.guid == guid) {
        // A device with the same guid already exists, return it
        existing_device.id
    } else {
        // No device with the same guid exists, insert the new device
        let new_id = devices.id_factory.next_id();
        devices.entries.insert(new_id, Device::new(new_id, guid.to_string(), name.to_string()));
        new_id
    }
}

/// Remove a device from the simulation.
///
/// Called when the last chip for the device is removed.
fn remove_device(
    guard: &mut RwLockWriteGuard<Devices>,
    id: DeviceIdentifier,
) -> Result<(), String> {
    guard.entries.remove(&id).ok_or(format!("Error removing device with id {id}"))?;
    if guard.entries.is_empty() {
        guard.idle_since = Some(Instant::now());
    }
    Ok(())
}

/// Remove a chip from a device.
///
/// Called when the packet transport for the chip shuts down.
pub fn remove_chip(device_id: DeviceIdentifier, chip_id: ChipIdentifier) -> Result<(), String> {
    let result = {
        let mut _facade_id_option: Option<FacadeIdentifier> = None;
        let mut _chip_kind = ProtoChipKind::UNSPECIFIED;
        let devices_arc = clone_devices();
        let mut devices = devices_arc.write().unwrap();
        let is_empty = match devices.entries.entry(device_id) {
            Entry::Occupied(mut entry) => {
                let device = entry.get_mut();
                (_facade_id_option, _chip_kind) = device.remove_chip(chip_id)?;
                device.chips.is_empty()
            }
            Entry::Vacant(_) => return Err(format!("RemoveChip device id {device_id} not found")),
        };
        if is_empty {
            remove_device(&mut devices, device_id)?;
        }
        Ok((_facade_id_option, _chip_kind))
    };
    match result {
        Ok((facade_id_option, chip_kind)) => {
            match facade_id_option {
                Some(facade_id) => match chip_kind {
                    ProtoChipKind::BLUETOOTH => {
                        bluetooth_facade::bluetooth_remove(facade_id);
                    }
                    ProtoChipKind::WIFI => {
                        wifi_facade::wifi_remove(facade_id);
                    }
                    _ => Err(format!("Unknown chip kind: {:?}", chip_kind))?,
                },
                None => Err(format!(
                    "Facade Id hasn't been added yet to frontend resource for chip_id: {chip_id}"
                ))?,
            }
            info!("Removed Chip: device_id: {device_id}, chip_id: {chip_id}");
            resource::clone_events()
                .lock()
                .unwrap()
                .publish(Event::ChipRemoved { id: chip_id as u32 });
            update_captures();
            Ok(())
        }
        Err(err) => {
            error!("Failed to remove chip: device_id: {device_id}, chip_id: {chip_id}");
            Err(err)
        }
    }
}

/// A RemoveChip function for Rust Device API.
/// The backend gRPC code will be invoking this method.
pub fn remove_chip_cxx(device_id: u32, chip_id: u32) {
    let _ = remove_chip(device_id as i32, chip_id as i32);
}

// lock the devices, find the id and call the patch function
#[allow(dead_code)]
fn patch_device(id_option: Option<DeviceIdentifier>, patch_json: &str) -> Result<(), String> {
    let mut patch_device_request = PatchDeviceRequest::new();
    if merge_from_str(&mut patch_device_request, patch_json).is_ok() {
        let devices_arc = clone_devices();
        let mut devices = devices_arc.write().unwrap();
        let proto_device = patch_device_request.device;
        match id_option {
            Some(id) => match devices.entries.get_mut(&id) {
                Some(device) => device.patch(&proto_device),
                None => Err(format!("No such device with id {id}")),
            },
            None => {
                let mut multiple_matches = false;
                let mut target: Option<&mut Device> = None;
                for device in devices.entries.values_mut() {
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
    let devices_arc = clone_devices();
    let devices = devices_arc.read().unwrap();
    let a = devices
        .entries
        .get(&id)
        .map(|device_ref| device_ref.position.clone())
        .ok_or(format!("No such device with id {id}"))?;
    let b = devices
        .entries
        .get(&other_id)
        .map(|device_ref| device_ref.position.clone())
        .ok_or(format!("No such device with id {other_id}"))?;
    Ok(distance(&a, &b))
}

/// A GetDistance function for Rust Device API.
/// The backend gRPC code will be invoking this method.
pub fn get_distance_cxx(a: u32, b: u32) -> f32 {
    match get_distance(a as i32, b as i32) {
        Ok(distance) => distance,
        Err(err) => {
            error!("get_distance Error: {err}");
            0.0
        }
    }
}

pub fn get_devices() -> Result<ProtoScene, String> {
    let mut scene = ProtoScene::new();
    // iterate over the devices and add each to the scene
    let devices_arc = clone_devices();
    let devices = devices_arc.read().unwrap();
    for device in devices.entries.values() {
        scene.devices.push(device.get()?);
    }
    Ok(scene)
}

#[allow(dead_code)]
fn reset(id: DeviceIdentifier) -> Result<(), String> {
    let devices_arc = clone_devices();
    let mut devices = devices_arc.write().unwrap();
    match devices.entries.get_mut(&id) {
        Some(device) => device.reset(),
        None => Err(format!("No such device with id {id}")),
    }
}

#[allow(dead_code)]
fn reset_all() -> Result<(), String> {
    let devices_arc = clone_devices();
    let mut devices = devices_arc.write().unwrap();
    for device in devices.entries.values_mut() {
        device.reset()?;
    }
    Ok(())
}

/// Return true if netsimd is idle for 5 minutes
pub fn is_shutdown_time_cxx() -> bool {
    let devices_arc = clone_devices();
    let devices = devices_arc.read().unwrap();
    match devices.idle_since {
        Some(idle_since) => {
            IDLE_SECS_FOR_SHUTDOWN.checked_sub(idle_since.elapsed().as_secs()).is_none()
        }
        None => false,
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
    let devices_arc = clone_devices();
    let devices = devices_arc.read().unwrap();
    // Instantiate ListDeviceResponse and add Devices
    let mut response = ListDeviceResponse::new();
    for device in devices.entries.values() {
        response.devices.push(device.get().unwrap());
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

/// Get Facade ID from given chip_id
pub fn get_facade_id(chip_id: i32) -> Result<u32, String> {
    let devices_arc = clone_devices();
    let devices = devices_arc.read().unwrap();
    for device in devices.entries.values() {
        for (id, chip) in &device.chips {
            if *id == chip_id {
                return chip.facade_id.ok_or(format!(
                    "Facade Id hasn't been added yet to frontend resource for chip_id: {chip_id}"
                ));
            }
        }
    }
    Err(format!("Cannot find facade_id for {chip_id}"))
}

#[cfg(test)]
mod tests {
    use std::{
        io::Cursor,
        sync::{Mutex, Once},
        time::Duration,
    };

    use frontend_proto::model::{Device as ProtoDevice, Orientation as ProtoOrientation, State};
    use lazy_static::lazy_static;
    use netsim_common::util::netsim_logger::init_for_test;
    use protobuf_json_mapping::print_to_string;

    use crate::http_server::server_response::ServerResponseWriter;

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
            let devices_arc = clone_devices();
            let mut devices = devices_arc.write().unwrap();
            super::get_or_create_device(&mut devices, self.device_guid, self.device_name)
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
        let devices_arc = clone_devices();
        let mut devices = devices_arc.write().unwrap();
        devices.entries = BTreeMap::new();
        devices.id_factory = IdFactory::new(1000, 1);
        devices.idle_since = Some(Instant::now());
        crate::devices::chip::refresh_resource();
        crate::bluetooth::refresh_resource();
        crate::wifi::refresh_resource();
    }

    /// helper function for traveling back n seconds for idle_since
    fn travel_back_n_seconds_from_now(n: u64) {
        let devices_arc = clone_devices();
        let mut devices = devices_arc.write().unwrap();
        devices.idle_since = Some(Instant::now() - Duration::from_secs(n));
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
        assert_eq!(clone_devices().read().unwrap().entries.len(), 1);
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

    #[test]
    fn test_get_facade_id() {
        // Avoiding Interleaving Operations
        let _lock = MUTEX.lock().unwrap();

        // Initializing Logger
        logger_setup();

        // Add bt, wifi chips of the same device and bt chip of second device
        refresh_resource();
        let bt_chip_params = test_chip_1_bt();
        let bt_chip_result = bt_chip_params.add_chip().unwrap();
        let wifi_chip_params = test_chip_1_wifi();
        let wifi_chip_result = wifi_chip_params.add_chip().unwrap();
        let bt_chip_2_params = test_chip_2_bt();
        let bt_chip_2_result = bt_chip_2_params.add_chip().unwrap();

        // Invoke get_facade_id from first bt chip
        match get_facade_id(bt_chip_result.chip_id) {
            Ok(facade_id) => assert_eq!(facade_id, 0),
            Err(err) => {
                error!("{err}");
                unreachable!();
            }
        }

        // Invoke get_facade_id from first wifi chip
        match get_facade_id(wifi_chip_result.chip_id) {
            Ok(facade_id) => assert_eq!(facade_id, 0),
            Err(err) => {
                error!("{err}");
                unreachable!();
            }
        }

        // Invoke get_facade_id from second bt chip
        match get_facade_id(bt_chip_2_result.chip_id) {
            Ok(facade_id) => assert_eq!(facade_id, 1),
            Err(err) => {
                error!("{err}");
                unreachable!();
            }
        }
    }

    #[test]
    fn test_is_shutdown_time_cxx() {
        // Avoiding Interleaving Operations
        let _lock = MUTEX.lock().unwrap();

        // Initializing Logger
        logger_setup();

        // Refresh Resource
        refresh_resource();

        // Set the idle_since value to more than 15 seconds before current time
        travel_back_n_seconds_from_now(16);
        assert!(is_shutdown_time_cxx());

        // Set the idle_since value to less than 15 seconds before current time
        travel_back_n_seconds_from_now(14);
        assert!(!is_shutdown_time_cxx());

        // Refresh Resource again
        refresh_resource();

        // Add a device and check if idle_since is None
        let _ = test_chip_1_bt().add_chip();
        let devices_arc = clone_devices();
        let devices = devices_arc.read().unwrap();
        assert!(devices.idle_since.is_none());
        assert!(!is_shutdown_time_cxx());
    }

    fn list_request() -> HttpRequest {
        HttpRequest {
            method: "GET".to_string(),
            uri: "/v1/devices".to_string(),
            version: "1.1".to_string(),
            headers: HttpHeaders::new(),
            body: b"".to_vec(),
        }
    }

    #[test]
    fn test_handle_device() {
        // Avoiding Interleaving Operations
        let _lock = MUTEX.lock().unwrap();

        // Initializing Logger
        logger_setup();

        // Refresh Resource
        refresh_resource();

        // Add bt, wifi chips of the same device and bt chip of second device
        let bt_chip_params = test_chip_1_bt();
        let _bt_chip_result = bt_chip_params.add_chip().unwrap();
        let wifi_chip_params = test_chip_1_wifi();
        let _wifi_chip_result = wifi_chip_params.add_chip().unwrap();
        let bt_chip_2_params = test_chip_2_bt();
        let _bt_chip_2_result = bt_chip_2_params.add_chip().unwrap();

        // ListDevice Testing

        // Initialize request for ListDevice
        let request = list_request();

        // Initialize writer
        let mut stream = Cursor::new(Vec::new());
        let mut writer = ServerResponseWriter::new(&mut stream);

        // Perform ListDevice
        handle_device(&request, "", &mut writer);

        // Check the response for ListDevice
        let expected = include_bytes!("test/initial.txt");
        let actual = stream.get_ref();
        assert_eq!(actual, expected);

        // PatchDevice Testing

        // Initialize request for PatchDevice
        // The patch body will change the visibility and position of the first device.
        let request = HttpRequest {
            method: "PATCH".to_string(),
            uri: "/v1/devices".to_string(),
            version: "1.1".to_string(),
            headers: HttpHeaders::new(),
            body: include_bytes!("test/patch_body.txt").to_vec(),
        };

        // Initialize writer
        let mut stream = Cursor::new(Vec::new());
        let mut writer = ServerResponseWriter::new(&mut stream);

        // Perform PatchDevice
        handle_device(&request, "", &mut writer);

        // Initialize writer
        let mut stream = Cursor::new(Vec::new());
        let mut writer = ServerResponseWriter::new(&mut stream);

        // Perform ListDevice
        let request = list_request();
        handle_device(&request, "", &mut writer);

        // Check the response for ListDevice
        let expected = include_bytes!("test/post_patch.txt");
        let actual = stream.get_ref();
        assert_eq!(actual, expected);

        // ResetDevice Testing

        // Initialize request for ResetDevice
        let request = HttpRequest {
            method: "PUT".to_string(),
            uri: "/v1/devices".to_string(),
            version: "1.1".to_string(),
            headers: HttpHeaders::new(),
            body: b"".to_vec(),
        };

        // Initialize writer
        let mut stream = Cursor::new(Vec::new());
        let mut writer = ServerResponseWriter::new(&mut stream);

        // Perform ResetDevice
        handle_device(&request, "", &mut writer);

        // Initialize writer
        let mut stream = Cursor::new(Vec::new());
        let mut writer = ServerResponseWriter::new(&mut stream);

        // Perform ResetDevice
        let request = list_request();
        handle_device(&request, "", &mut writer);

        // Check the response for ListDevice
        let expected = include_bytes!("test/initial.txt");
        let actual = stream.get_ref();
        assert_eq!(actual, expected);
    }

    // Helper function for regenerating golden files
    fn regenerate_golden_files_helper() {
        use std::io::Write;

        // Write initial state of the test case (2 bt chip and 1 wifi chip)
        let mut file = std::fs::File::create("src/devices/test/initial.txt").unwrap();
        let initial = b"HTTP/1.1 200 OK\r\nContent-Type: text/json\r\nContent-Length: 783\r\n\r\n{\"devices\": [{\"id\": 1000, \"name\": \"test-device-name-1\", \"visible\": \"ON\", \"position\": {\"x\": 0.0, \"y\": 0.0, \"z\": 0.0}, \"orientation\": {\"yaw\": 0.0, \"pitch\": 0.0, \"roll\": 0.0}, \"chips\": [{\"kind\": \"BLUETOOTH\", \"id\": 1000, \"name\": \"bt_chip_name\", \"manufacturer\": \"netsim\", \"productName\": \"netsim_bt\", \"bt\": {}}, {\"kind\": \"WIFI\", \"id\": 1001, \"name\": \"bt_chip_name\", \"manufacturer\": \"netsim\", \"productName\": \"netsim_bt\", \"wifi\": {\"state\": \"UNKNOWN\", \"range\": 0.0, \"txCount\": 0, \"rxCount\": 0}}]}, {\"id\": 1001, \"name\": \"test-device-name-2\", \"visible\": \"ON\", \"position\": {\"x\": 0.0, \"y\": 0.0, \"z\": 0.0}, \"orientation\": {\"yaw\": 0.0, \"pitch\": 0.0, \"roll\": 0.0}, \"chips\": [{\"kind\": \"BLUETOOTH\", \"id\": 1002, \"name\": \"bt_chip_name\", \"manufacturer\": \"netsim\", \"productName\": \"netsim_bt\", \"bt\": {}}]}]}";
        file.write_all(initial).unwrap();

        // Write the body of the patch request
        let mut file = std::fs::File::create("src/devices/test/patch_body.txt").unwrap();
        let patch_body = b"{\"device\": {\"name\": \"test-device-name-1\", \"visible\": \"OFF\", \"position\": {\"x\": 1.0, \"y\": 1.0, \"z\": 1.0}}}";
        file.write_all(patch_body).unwrap();

        // Write post-patch state of the test case (after PatchDevice)
        let mut file = std::fs::File::create("src/devices/test/post_patch.txt").unwrap();
        let post_patch = b"HTTP/1.1 200 OK\r\nContent-Type: text/json\r\nContent-Length: 784\r\n\r\n{\"devices\": [{\"id\": 1000, \"name\": \"test-device-name-1\", \"visible\": \"OFF\", \"position\": {\"x\": 1.0, \"y\": 1.0, \"z\": 1.0}, \"orientation\": {\"yaw\": 0.0, \"pitch\": 0.0, \"roll\": 0.0}, \"chips\": [{\"kind\": \"BLUETOOTH\", \"id\": 1000, \"name\": \"bt_chip_name\", \"manufacturer\": \"netsim\", \"productName\": \"netsim_bt\", \"bt\": {}}, {\"kind\": \"WIFI\", \"id\": 1001, \"name\": \"bt_chip_name\", \"manufacturer\": \"netsim\", \"productName\": \"netsim_bt\", \"wifi\": {\"state\": \"UNKNOWN\", \"range\": 0.0, \"txCount\": 0, \"rxCount\": 0}}]}, {\"id\": 1001, \"name\": \"test-device-name-2\", \"visible\": \"ON\", \"position\": {\"x\": 0.0, \"y\": 0.0, \"z\": 0.0}, \"orientation\": {\"yaw\": 0.0, \"pitch\": 0.0, \"roll\": 0.0}, \"chips\": [{\"kind\": \"BLUETOOTH\", \"id\": 1002, \"name\": \"bt_chip_name\", \"manufacturer\": \"netsim\", \"productName\": \"netsim_bt\", \"bt\": {}}]}]}";
        file.write_all(post_patch).unwrap();
    }

    /// This is not a test function
    /// Uncomment the helper function and run the test to regenerate golden files.
    #[test]
    fn regenerate_golden_files() {
        // Avoiding Interleaving Operations
        let _lock = MUTEX.lock().unwrap();

        // regenerate_golden_files_helper();
    }
}
