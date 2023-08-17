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
use crate::devices::device::AddChipResult;
use crate::devices::device::Device;
use crate::events::Event;
use crate::ffi::CxxServerResponseWriter;
use crate::http_server::server_response::ResponseWritable;
use crate::resource;
use crate::resource::clone_devices;
use crate::wifi as wifi_facade;
use crate::CxxServerResponseWriterWrapper;
use cxx::CxxString;
use frontend_proto::common::ChipKind as ProtoChipKind;
use frontend_proto::frontend::CreateDeviceRequest;
use frontend_proto::frontend::CreateDeviceResponse;
use frontend_proto::frontend::ListDeviceResponse;
use frontend_proto::frontend::PatchDeviceRequest;
use frontend_proto::model::chip_create::Chip as ProtoBuiltin;
use frontend_proto::model::ChipCreate;
use frontend_proto::model::Position as ProtoPosition;
use frontend_proto::model::Scene as ProtoScene;
use http::Request;
use http::Version;
use log::{info, warn};
use protobuf::MessageField;
use protobuf_json_mapping::merge_from_str;
use protobuf_json_mapping::print_to_string;
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
///
/// TODO: Replace the parameter of add_chip with a single protobuf
pub fn add_chip(
    device_guid: &str,
    device_name: &str,
    chip_create_proto: &ChipCreate,
) -> Result<AddChipResult, String> {
    let chip_kind = chip_create_proto.kind.enum_value_or(ProtoChipKind::UNSPECIFIED);
    let result = {
        let devices_arc = clone_devices();
        let mut devices = devices_arc.write().unwrap();
        devices.idle_since = None;
        let (device_id, _) =
            get_or_create_device(&mut devices, Some(device_guid), Some(device_name));

        let chip_name = (chip_create_proto.name != String::default())
            .then_some(chip_create_proto.name.as_str());
        // This is infrequent, so we can afford to do another lookup for the device.
        devices
            .entries
            .get_mut(&device_id)
            .ok_or(format!("Device not found for device_id: {device_id}"))?
            .add_chip(
                chip_kind,
                &chip_create_proto.address,
                chip_name,
                &chip_create_proto.manufacturer,
                &chip_create_proto.product_name,
            )
    };

    // Device resource is no longer locked
    match result {
        // id_tuple = (DeviceIdentifier, ChipIdentifier)
        Ok((device_id, chip_id)) => {
            let facade_id = match chip_kind {
                ProtoChipKind::BLUETOOTH => {
                    bluetooth_facade::bluetooth_add(device_id, &chip_create_proto.address)
                }
                ProtoChipKind::BLUETOOTH_BEACON => bluetooth_facade::bluetooth_beacon_add(
                    device_id,
                    String::from(device_name),
                    chip_id,
                    chip_create_proto,
                )?,
                ProtoChipKind::WIFI => wifi_facade::wifi_add(device_id),
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
            resource::clone_events().lock().unwrap().publish(Event::ChipAdded {
                chip_id,
                chip_kind,
                facade_id,
                device_name: device_name.to_string(),
            });
            Ok(AddChipResult { device_id, chip_id, facade_id })
        }
        Err(err) => {
            warn!(
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
    chip_address: &str,
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
    let chip_create_proto = ChipCreate {
        kind: chip_kind_proto.into(),
        address: chip_address.to_string(),
        name: chip_name.to_string(),
        manufacturer: chip_manufacturer.to_string(),
        product_name: chip_product_name.to_string(),
        ..Default::default()
    };
    match add_chip(device_guid, device_name, &chip_create_proto) {
        Ok(result) => Box::new(AddChipResultCxx {
            device_id: result.device_id,
            chip_id: result.chip_id,
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
/// Returns a (device_id, device_name) pair.
fn get_or_create_device(
    devices: &mut Devices,
    guid: Option<&str>,
    name: Option<&str>,
) -> (DeviceIdentifier, String) {
    // Check if a device with the same guid already exists and if so, return it
    if let Some(guid) = guid {
        if let Some(existing_device) = devices.entries.values().find(|d| d.guid == *guid) {
            return (existing_device.id, existing_device.name.clone());
        }
    }

    // A new device needs to be created and inserted
    let new_id = devices.id_factory.next_id();
    let default = format!("device-{}", new_id);
    let name = name.unwrap_or(&default);
    devices.entries.insert(
        new_id,
        Device::new(new_id, String::from(guid.unwrap_or(&default)), String::from(name)),
    );

    (new_id, String::from(name))
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
            resource::clone_events().lock().unwrap().publish(Event::ChipRemoved { chip_id });
            Ok(())
        }
        Err(err) => {
            warn!("Failed to remove chip: device_id: {device_id}, chip_id: {chip_id}");
            Err(err)
        }
    }
}

/// A RemoveChip function for Rust Device API.
/// The backend gRPC code will be invoking this method.
pub fn remove_chip_cxx(device_id: u32, chip_id: u32) {
    let _ = remove_chip(device_id, chip_id);
}

/// Create a device from a CreateDeviceRequest json.
/// Uses a default name if none is provided.
/// Returns an error if the device already exists.
pub fn create_device(create_json: &str) -> Result<DeviceIdentifier, String> {
    let mut create_device_request = CreateDeviceRequest::new();
    if merge_from_str(&mut create_device_request, create_json).is_err() {
        return Err(format!(
            "failed to create device: incorrectly formatted create json: {}",
            create_json
        ));
    }

    let new_device = create_device_request.device;
    let devices_arc = clone_devices();
    let mut devices = devices_arc.write().unwrap();
    // Check if specified device name is already mapped.
    if new_device.name != String::default()
        && devices.entries.values().any(|d| d.guid == new_device.name)
    {
        return Err(String::from("failed to create device: device already exists"));
    }

    if new_device.chips.is_empty() {
        return Err(String::from("failed to create device: device must contain at least 1 chip"));
    }
    new_device.chips.iter().try_for_each(|chip| match chip.chip {
        Some(ProtoBuiltin::BleBeacon(_)) => Ok(()),
        Some(_) => Err(format!("failed to create device: chip {} was not a built-in", chip.name)),
        None => Err(format!("failed to create device: chip {} was missing a radio", chip.name)),
    })?;

    let device_name = (new_device.name != String::default()).then_some(new_device.name.as_str());
    let (device_id, device_name) = get_or_create_device(&mut devices, device_name, device_name);

    // Release devices lock so that add_chip can take it.
    drop(devices);
    new_device
        .chips
        .iter()
        .try_for_each(|chip| add_chip(&device_name, &device_name, chip).map(|_| ()))?;

    resource::clone_events()
        .lock()
        .unwrap()
        .publish(Event::DeviceAdded { id: device_id, name: device_name });

    Ok(device_id)
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
                Some(device) => {
                    let result = device.patch(&proto_device);
                    if result.is_ok() {
                        // Publish Device Patched event
                        resource::clone_events()
                            .lock()
                            .unwrap()
                            .publish(Event::DevicePatched { id, name: device.name.clone() });
                    }
                    result
                }
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
                    Some(device) => {
                        let result = device.patch(&proto_device);
                        if result.is_ok() {
                            // Publish Device Patched event
                            resource::clone_events().lock().unwrap().publish(
                                Event::DevicePatched { id: device.id, name: device.name.clone() },
                            );
                        }
                        result
                    }
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
    match get_distance(a, b) {
        Ok(distance) => distance,
        Err(err) => {
            warn!("get_distance Error: {err}");
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

/// Return true if netsimd is idle for a certain duration.
pub fn is_shutdown_time() -> bool {
    let devices_arc = clone_devices();
    let devices = devices_arc.read().unwrap();
    match devices.idle_since {
        Some(idle_since) => {
            IDLE_SECS_FOR_SHUTDOWN.checked_sub(idle_since.elapsed().as_secs()).is_none()
        }
        None => false,
    }
}

fn handle_device_create(writer: ResponseWritable, create_json: &str) {
    let mut response = CreateDeviceResponse::new();

    let mut collate_results = || {
        let id = create_device(create_json)?;

        let devices_arc = clone_devices();
        let devices = devices_arc.read().unwrap();
        let device_proto = devices.entries.get(&id).ok_or("failed to create device")?.get()?;
        response.device = MessageField::some(device_proto);
        print_to_string(&response).map_err(|_| String::from("failed to convert device to json"))
    };

    match collate_results() {
        Ok(response) => writer.put_ok("text/json", &response, vec![]),
        Err(err) => writer.put_error(404, err.as_str()),
    }
}

/// Performs PatchDevice to patch a single device
fn handle_device_patch(writer: ResponseWritable, id: Option<DeviceIdentifier>, patch_json: &str) {
    match patch_device(id, patch_json) {
        Ok(()) => writer.put_ok("text/plain", "Device Patch Success", vec![]),
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
        writer.put_ok("text/json", &json_response, vec![])
    } else {
        writer.put_error(404, "proto to JSON mapping failure")
    }
}

/// Performs ResetDevice for all devices
fn handle_device_reset(writer: ResponseWritable) {
    match reset_all() {
        Ok(()) => writer.put_ok("text/plain", "Device Reset Success", vec![]),
        Err(err) => writer.put_error(404, err.as_str()),
    }
}

/// The Rust device handler used directly by Http frontend or handle_device_cxx for LIST, GET, and PATCH
pub fn handle_device(request: &Request<Vec<u8>>, param: &str, writer: ResponseWritable) {
    // Route handling
    if request.uri() == "/v1/devices" {
        // Routes with ID not specified
        match request.method().as_str() {
            "GET" => {
                handle_device_list(writer);
            }
            "PUT" => {
                handle_device_reset(writer);
            }
            "PATCH" => {
                let body = request.body();
                let patch_json = String::from_utf8(body.to_vec()).unwrap();
                handle_device_patch(writer, None, patch_json.as_str());
            }
            "POST" => {
                let body = &request.body();
                let create_json = String::from_utf8(body.to_vec()).unwrap();
                handle_device_create(writer, create_json.as_str());
            }
            _ => writer.put_error(404, "Not found."),
        }
    } else {
        // Routes with ID specified
        match request.method().as_str() {
            "PATCH" => {
                let id = match param.parse::<u32>() {
                    Ok(num) => num,
                    Err(_) => {
                        writer.put_error(404, "Incorrect Id type for devices, ID should be u32.");
                        return;
                    }
                };
                let body = request.body();
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
    let mut builder = Request::builder().method(method.as_str());
    if param.is_empty() {
        builder = builder.uri("/v1/devices");
    } else {
        builder = builder.uri(format!("/v1/devices/{}", param));
    }
    builder = builder.version(Version::HTTP_11);
    let request = match builder.body(body.as_bytes().to_vec()) {
        Ok(request) => request,
        Err(err) => {
            warn!("{err:?}");
            return;
        }
    };
    handle_device(
        &request,
        param.as_str(),
        &mut CxxServerResponseWriterWrapper { writer: responder },
    )
}

/// Get Facade ID from given chip_id
pub fn get_facade_id(chip_id: u32) -> Result<u32, String> {
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
    use std::{sync::Once, thread, time::Duration};

    use frontend_proto::model::{
        Device as ProtoDevice, DeviceCreate as ProtoDeviceCreate, Orientation as ProtoOrientation,
        State,
    };
    use netsim_common::util::netsim_logger::init_for_test;
    use protobuf_json_mapping::print_to_string;

    use super::*;

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
    struct TestChipParameters {
        device_guid: String,
        device_name: String,
        chip_kind: ProtoChipKind,
        chip_name: String,
        chip_manufacturer: String,
        chip_product_name: String,
    }

    impl TestChipParameters {
        fn add_chip(&self) -> Result<AddChipResult, String> {
            let chip_create_proto = ChipCreate {
                kind: self.chip_kind.into(),
                name: self.chip_name.to_string(),
                manufacturer: self.chip_manufacturer.to_string(),
                product_name: self.chip_product_name.to_string(),
                ..Default::default()
            };
            super::add_chip(&self.device_guid, &self.device_name, &chip_create_proto)
        }

        fn get_or_create_device(&self) -> DeviceIdentifier {
            let devices_arc = clone_devices();
            let mut devices = devices_arc.write().unwrap();
            super::get_or_create_device(
                &mut devices,
                Some(&self.device_guid),
                Some(&self.device_name),
            )
            .0
        }
    }

    /// helper function for test cases to instantiate ProtoPosition
    fn new_position(x: f32, y: f32, z: f32) -> ProtoPosition {
        ProtoPosition { x, y, z, ..Default::default() }
    }

    fn new_orientation(yaw: f32, pitch: f32, roll: f32) -> ProtoOrientation {
        ProtoOrientation { yaw, pitch, roll, ..Default::default() }
    }

    /// helper function for traveling back n seconds for idle_since
    fn travel_back_n_seconds_from_now(n: u64) {
        let devices_arc = clone_devices();
        let mut devices = devices_arc.write().unwrap();
        devices.idle_since = Some(Instant::now() - Duration::from_secs(n));
    }

    fn test_chip_1_bt() -> TestChipParameters {
        TestChipParameters {
            device_guid: format!("guid-fs-1-{:?}", thread::current().id()),
            device_name: format!("test-device-name-1-{:?}", thread::current().id()),
            chip_kind: ProtoChipKind::BLUETOOTH,
            chip_name: "bt_chip_name".to_string(),
            chip_manufacturer: "netsim".to_string(),
            chip_product_name: "netsim_bt".to_string(),
        }
    }

    fn test_chip_1_wifi() -> TestChipParameters {
        TestChipParameters {
            device_guid: format!("guid-fs-1-{:?}", thread::current().id()),
            device_name: format!("test-device-name-1-{:?}", thread::current().id()),
            chip_kind: ProtoChipKind::WIFI,
            chip_name: "wifi_chip_name".to_string(),
            chip_manufacturer: "netsim".to_string(),
            chip_product_name: "netsim_wifi".to_string(),
        }
    }

    fn test_chip_2_bt() -> TestChipParameters {
        TestChipParameters {
            device_guid: format!("guid-fs-2-{:?}", thread::current().id()),
            device_name: format!("test-device-name-2-{:?}", thread::current().id()),
            chip_kind: ProtoChipKind::BLUETOOTH,
            chip_name: "bt_chip_name".to_string(),
            chip_manufacturer: "netsim".to_string(),
            chip_product_name: "netsim_bt".to_string(),
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
        // Initializing Logger
        logger_setup();

        // Adding a chip
        let chip_params = test_chip_1_bt();
        let chip_result = chip_params.add_chip().unwrap();
        match clone_devices().read().unwrap().entries.get(&chip_result.device_id) {
            Some(device) => {
                let chip = device.chips.get(&chip_result.chip_id).unwrap();
                assert_eq!(chip_params.chip_kind, chip.kind);
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
        // Initializing Logger
        logger_setup();

        // Creating a device and getting device
        let bt_chip_params = test_chip_1_bt();
        let device_id_1 = bt_chip_params.get_or_create_device();
        let wifi_chip_params = test_chip_1_wifi();
        let device_id_2 = wifi_chip_params.get_or_create_device();
        assert_eq!(device_id_1, device_id_2);
    }

    #[test]
    fn test_patch_device() {
        // Initializing Logger
        logger_setup();

        // Patching device position and orientation by id
        let chip_params = test_chip_1_bt();
        let chip_result = chip_params.add_chip().unwrap();
        let mut patch_device_request = PatchDeviceRequest::new();
        let mut proto_device = ProtoDevice::new();
        let request_position = new_position(1.1, 2.2, 3.3);
        let request_orientation = new_orientation(4.4, 5.5, 6.6);
        proto_device.name = chip_params.device_name;
        proto_device.visible = State::OFF.into();
        proto_device.position = Some(request_position.clone()).into();
        proto_device.orientation = Some(request_orientation.clone()).into();
        patch_device_request.device = Some(proto_device.clone()).into();
        let patch_json = print_to_string(&patch_device_request).unwrap();
        patch_device(Some(chip_result.device_id), patch_json.as_str()).unwrap();
        match clone_devices().read().unwrap().entries.get(&chip_result.device_id) {
            Some(device) => {
                assert_eq!(device.position.x, request_position.x);
                assert_eq!(device.position.y, request_position.y);
                assert_eq!(device.position.z, request_position.z);
                assert_eq!(device.orientation.yaw, request_orientation.yaw);
                assert_eq!(device.orientation.pitch, request_orientation.pitch);
                assert_eq!(device.orientation.roll, request_orientation.roll);
                assert_eq!(device.visible, State::OFF);
            }
            None => unreachable!(),
        }

        // Patch device by name with substring match
        proto_device.name = format!("test-device-name-1-{:?}", thread::current().id());
        patch_device_request.device = Some(proto_device).into();
        let patch_json = print_to_string(&patch_device_request).unwrap();
        assert!(patch_device(None, patch_json.as_str()).is_ok());
    }

    #[test]
    fn test_patch_error() {
        // Initializing Logger
        logger_setup();

        // Patch Error Testing
        let bt_chip_params = test_chip_1_bt();
        let bt_chip2_params = test_chip_2_bt();
        let bt_chip_result = bt_chip_params.add_chip().unwrap();
        bt_chip2_params.add_chip().unwrap();

        // Incorrect value type
        let error_json = format!(
            "{{\"device\": {{\"name\": \"test-device-name-1-{:?}\", \"position\": 1.1}}}}",
            thread::current().id()
        );
        let patch_result = patch_device(Some(bt_chip_result.device_id), error_json.as_str());
        assert!(patch_result.is_err());
        assert_eq!(
            patch_result.unwrap_err(),
            format!("Incorrect format of patch json {}", error_json)
        );

        // Incorrect key
        let error_json = format!(
            "{{\"device\": {{\"name\": \"test-device-name-1-{:?}\", \"hello\": \"world\"}}}}",
            thread::current().id()
        );
        let patch_result = patch_device(Some(bt_chip_result.device_id), error_json.as_str());
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
        // Initializing Logger
        logger_setup();

        // Adding two chips of the same device
        let bt_chip_params = test_chip_1_bt();
        let wifi_chip_params = test_chip_1_wifi();
        let bt_chip_result = bt_chip_params.add_chip().unwrap();
        let wifi_chip_result = wifi_chip_params.add_chip().unwrap();
        assert_eq!(bt_chip_result.device_id, wifi_chip_result.device_id);
        let binding = clone_devices();
        let binding = binding.read().unwrap();
        let device = binding.entries.get(&bt_chip_result.device_id).unwrap();
        assert_eq!(device.id, bt_chip_result.device_id);
        assert_eq!(device.name, bt_chip_params.device_name);
        assert_eq!(device.chips.len(), 2);
        for chip in device.chips.values() {
            assert!(chip.id == bt_chip_result.chip_id || chip.id == wifi_chip_result.chip_id);
            if chip.id == bt_chip_result.chip_id {
                assert_eq!(chip.kind, ProtoChipKind::BLUETOOTH);
            } else if chip.id == wifi_chip_result.chip_id {
                assert_eq!(chip.kind, ProtoChipKind::WIFI);
            } else {
                unreachable!();
            }
        }
    }

    #[test]
    fn test_reset() {
        // Initializing Logger
        logger_setup();

        // Patching Device and Resetting scene
        let chip_params = test_chip_1_bt();
        let chip_result = chip_params.add_chip().unwrap();
        let mut patch_device_request = PatchDeviceRequest::new();
        let mut proto_device = ProtoDevice::new();
        let request_position = new_position(10.0, 20.0, 30.0);
        let request_orientation = new_orientation(1.0, 2.0, 3.0);
        proto_device.name = chip_params.device_name;
        proto_device.visible = State::OFF.into();
        proto_device.position = Some(request_position).into();
        proto_device.orientation = Some(request_orientation).into();
        patch_device_request.device = Some(proto_device).into();
        patch_device(
            Some(chip_result.device_id),
            print_to_string(&patch_device_request).unwrap().as_str(),
        )
        .unwrap();
        match clone_devices().read().unwrap().entries.get(&chip_result.device_id) {
            Some(device) => {
                assert_eq!(device.position.x, 10.0);
                assert_eq!(device.orientation.yaw, 1.0);
                assert_eq!(device.visible, State::OFF);
            }
            None => unreachable!(),
        }
        reset(chip_result.device_id).unwrap();
        match clone_devices().read().unwrap().entries.get(&chip_result.device_id) {
            Some(device) => {
                assert_eq!(device.position.x, 0.0);
                assert_eq!(device.position.y, 0.0);
                assert_eq!(device.position.z, 0.0);
                assert_eq!(device.orientation.yaw, 0.0);
                assert_eq!(device.orientation.pitch, 0.0);
                assert_eq!(device.orientation.roll, 0.0);
                assert_eq!(device.visible, State::ON);
            }
            None => unreachable!(),
        }
    }

    #[test]
    fn test_remove_chip() {
        // Initializing Logger
        logger_setup();

        // Add 2 chips of same device and 1 chip of different device
        let bt_chip_params = test_chip_1_bt();
        let wifi_chip_params = test_chip_1_wifi();
        let bt_chip_2_params = test_chip_2_bt();
        let bt_chip_result = bt_chip_params.add_chip().unwrap();
        let wifi_chip_result = wifi_chip_params.add_chip().unwrap();
        let bt_chip_2_result = bt_chip_2_params.add_chip().unwrap();

        // Remove a bt chip of first device
        remove_chip(bt_chip_result.device_id, bt_chip_result.chip_id).unwrap();
        match clone_devices().read().unwrap().entries.get(&bt_chip_result.device_id) {
            Some(device) => {
                assert_eq!(device.chips.len(), 1);
                assert_eq!(
                    device.chips.get(&wifi_chip_result.chip_id).unwrap().kind,
                    ProtoChipKind::WIFI
                );
            }
            None => unreachable!(),
        }

        // Remove a wifi chip of first device
        remove_chip(wifi_chip_result.device_id, wifi_chip_result.chip_id).unwrap();
        assert!(clone_devices().read().unwrap().entries.get(&wifi_chip_result.device_id).is_none());

        // Remove a bt chip of second device
        remove_chip(bt_chip_2_result.device_id, bt_chip_2_result.chip_id).unwrap();
        assert!(clone_devices().read().unwrap().entries.get(&bt_chip_2_result.device_id).is_none());
    }

    #[test]
    fn test_remove_chip_error() {
        // Initializing Logger
        logger_setup();

        // Add 2 chips of same device and 1 chip of different device
        let bt_chip_params = test_chip_1_bt();
        let bt_chip_result = bt_chip_params.add_chip().unwrap();

        // Invoke remove_chip with incorrect chip_id.
        match remove_chip(bt_chip_result.device_id, 9999) {
            Ok(_) => unreachable!(),
            Err(err) => assert_eq!(err, "RemoveChip chip id 9999 not found"),
        }

        // Invoke remove_chip with incorrect device_id
        match remove_chip(9999, bt_chip_result.chip_id) {
            Ok(_) => unreachable!(),
            Err(err) => assert_eq!(err, "RemoveChip device id 9999 not found"),
        }
        assert!(clone_devices().read().unwrap().entries.get(&bt_chip_result.device_id).is_some());
    }

    #[test]
    fn test_get_facade_id() {
        // Initializing Logger
        logger_setup();

        // Add bt, wifi chips of the same device and bt chip of second device
        let bt_chip_params = test_chip_1_bt();
        let bt_chip_result = bt_chip_params.add_chip().unwrap();
        let wifi_chip_params = test_chip_1_wifi();
        let wifi_chip_result = wifi_chip_params.add_chip().unwrap();
        let bt_chip_2_params = test_chip_2_bt();
        let bt_chip_2_result = bt_chip_2_params.add_chip().unwrap();

        // Invoke get_facade_id from first bt chip
        match get_facade_id(bt_chip_result.chip_id) {
            Ok(facade_id) => assert_eq!(facade_id, bt_chip_result.facade_id),
            Err(err) => {
                unreachable!("{err}");
            }
        }

        // Invoke get_facade_id from first wifi chip
        match get_facade_id(wifi_chip_result.chip_id) {
            Ok(facade_id) => assert_eq!(facade_id, wifi_chip_result.facade_id),
            Err(err) => {
                unreachable!("{err}");
            }
        }

        // Invoke get_facade_id from second bt chip
        match get_facade_id(bt_chip_2_result.chip_id) {
            Ok(facade_id) => assert_eq!(facade_id, bt_chip_2_result.facade_id),
            Err(err) => {
                unreachable!("{err}");
            }
        }
    }

    fn list_request() -> Request<Vec<u8>> {
        Request::builder()
            .method("GET")
            .uri("/v1/devices")
            .version(Version::HTTP_11)
            .body(Vec::<u8>::new())
            .unwrap()
    }

    use frontend_proto::model::chip::{
        bluetooth_beacon::AdvertiseData, bluetooth_beacon::AdvertiseSettings, BluetoothBeacon, Chip,
    };
    use frontend_proto::model::chip_create::{BluetoothBeaconCreate, Chip as BuiltChipProto};
    use frontend_proto::model::Chip as ChipProto;
    use frontend_proto::model::ChipCreate;
    use frontend_proto::model::Device as DeviceProto;
    use protobuf::{EnumOrUnknown, MessageField};

    fn get_test_create_device_request(device_name: Option<String>) -> CreateDeviceRequest {
        let beacon_proto = BluetoothBeaconCreate {
            settings: MessageField::some(AdvertiseSettings { ..Default::default() }),
            adv_data: MessageField::some(AdvertiseData { ..Default::default() }),
            ..Default::default()
        };

        let chip_proto = ChipCreate {
            name: String::from("test-beacon-chip"),
            kind: ProtoChipKind::BLUETOOTH_BEACON.into(),
            chip: Some(BuiltChipProto::BleBeacon(beacon_proto)),
            ..Default::default()
        };

        let device_proto = ProtoDeviceCreate {
            name: device_name.unwrap_or_default(),
            chips: vec![chip_proto],
            ..Default::default()
        };

        CreateDeviceRequest { device: MessageField::some(device_proto), ..Default::default() }
    }

    fn get_device_proto(id: DeviceIdentifier) -> DeviceProto {
        let devices = clone_devices();
        let devices_guard = devices.read().unwrap();
        let device =
            devices_guard.entries.get(&id).expect("could not find test bluetooth beacon device");

        let device_proto = device.get();
        assert!(device_proto.is_ok(), "{}", device_proto.unwrap_err());

        device_proto.unwrap()
    }

    #[test]
    fn test_create_device_succeeds() {
        logger_setup();

        let request = get_test_create_device_request(Some(format!(
            "bob-the-beacon-{:?}",
            thread::current().id()
        )));

        let id = create_device(&print_to_string(&request).unwrap());
        assert!(id.is_ok(), "{}", id.unwrap_err());
        let id = id.unwrap();

        let device_proto = get_device_proto(id);
        assert_eq!(request.device.name, device_proto.name);
        assert_eq!(1, device_proto.chips.len());
        assert_eq!(request.device.chips[0].name, device_proto.chips[0].name);
    }

    #[test]
    fn test_create_chipless_device_fails() {
        logger_setup();

        let request = CreateDeviceRequest {
            device: MessageField::some(ProtoDeviceCreate { ..Default::default() }),
            ..Default::default()
        };

        let id = create_device(&print_to_string(&request).unwrap());
        assert!(id.is_err(), "{}", id.unwrap());
    }

    #[test]
    fn test_create_radioless_device_fails() {
        logger_setup();

        let request = CreateDeviceRequest {
            device: MessageField::some(ProtoDeviceCreate {
                chips: vec![ChipCreate::default()],
                ..Default::default()
            }),
            ..Default::default()
        };

        let id = create_device(&print_to_string(&request).unwrap());
        assert!(id.is_err(), "{}", id.unwrap());
    }

    #[test]
    fn test_get_beacon_device() {
        logger_setup();

        let request = get_test_create_device_request(Some(format!(
            "bob-the-beacon-{:?}",
            thread::current().id()
        )));

        let id = create_device(&print_to_string(&request).unwrap());
        assert!(id.is_ok(), "{}", id.unwrap_err());
        let id = id.unwrap();

        let device_proto = get_device_proto(id);
        assert_eq!(1, device_proto.chips.len());
        assert!(device_proto.chips[0].chip.is_some());
        assert!(matches!(device_proto.chips[0].chip, Some(Chip::BleBeacon(_))));
    }

    #[test]
    fn test_create_device_default_name() {
        logger_setup();

        let request = get_test_create_device_request(None);

        let id = create_device(&print_to_string(&request).unwrap());
        assert!(id.is_ok(), "{}", id.unwrap_err());
        let id = id.unwrap();

        let device_proto = get_device_proto(id);
        assert_eq!(format!("device-{id}"), device_proto.name);
    }

    #[test]
    fn test_create_existing_device_fails() {
        logger_setup();

        let request = get_test_create_device_request(Some(format!(
            "existing-device-{:?}",
            thread::current().id()
        )));

        let request_json = print_to_string(&request).unwrap();

        let id = create_device(&request_json);
        assert!(id.is_ok(), "{}", id.unwrap_err());

        // Attempt to create the device again. This should fail because the devices have the same name.
        let id = create_device(&request_json);
        assert!(id.is_err());
    }

    #[test]
    fn test_patch_beacon_device() {
        logger_setup();

        let request = get_test_create_device_request(Some(format!(
            "bob-the-beacon-{:?}",
            thread::current().id()
        )));

        let id = create_device(&print_to_string(&request).unwrap());
        assert!(id.is_ok(), "{}", id.unwrap_err());
        let id = id.unwrap();

        let devices = clone_devices();
        let mut devices_guard = devices.write().unwrap();
        let device = devices_guard
            .entries
            .get_mut(&id)
            .expect("could not find test bluetooth beacon device");

        let device_proto = device.get();
        assert!(device_proto.is_ok(), "{}", device_proto.unwrap_err());
        let device_proto = device_proto.unwrap();

        let patch_result = device.patch(&DeviceProto {
            name: device_proto.name.clone(),
            id,
            chips: vec![ChipProto {
                name: request.device.chips[0].name.clone(),
                kind: EnumOrUnknown::new(ProtoChipKind::BLUETOOTH_BEACON),
                chip: Some(Chip::BleBeacon(BluetoothBeacon {
                    bt: MessageField::some(Default::default()),
                    ..Default::default()
                })),
                ..Default::default()
            }],
            ..Default::default()
        });
        assert!(patch_result.is_ok(), "{}", patch_result.unwrap_err());

        let patched_device = device.get();
        assert!(patched_device.is_ok(), "{}", patched_device.unwrap_err());
        let patched_device = patched_device.unwrap();
        assert_eq!(1, patched_device.chips.len());
        assert!(matches!(patched_device.chips[0].chip, Some(Chip::BleBeacon(_))));
    }
}
