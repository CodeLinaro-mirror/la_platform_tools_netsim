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
// The DeviceManager struct is a singleton for the devices collection.
//
// Additional functions are
// -- inactivity instant
// -- vending device identifiers

use super::chip;
use super::chip::ChipIdentifier;
use super::device::DeviceIdentifier;
use crate::devices::device::{AddChipResult, Device};
use crate::events;
use crate::events::{
    ChipAdded, ChipRemoved, DeviceAdded, DevicePatched, DeviceRemoved, Event, Events, ShutDown,
};
use crate::ffi::ffi_response_writable::CxxServerResponseWriter;
use crate::ffi::CxxServerResponseWriterWrapper;
use crate::http_server::server_response::ResponseWritable;
use crate::wireless;
use cxx::{CxxString, CxxVector};
use http::Request;
use http::Version;
use lazy_static::lazy_static;
use log::{info, warn};
use netsim_proto::common::ChipKind as ProtoChipKind;
use netsim_proto::configuration::Controller;
use netsim_proto::frontend::CreateDeviceRequest;
use netsim_proto::frontend::CreateDeviceResponse;
use netsim_proto::frontend::DeleteChipRequest;
use netsim_proto::frontend::ListDeviceResponse;
use netsim_proto::frontend::PatchDeviceRequest;
use netsim_proto::frontend::SubscribeDeviceRequest;
use netsim_proto::model::chip_create::Chip as ProtoBuiltin;
use netsim_proto::model::Position as ProtoPosition;
use netsim_proto::stats::NetsimRadioStats;
use protobuf::well_known_types::timestamp::Timestamp;
use protobuf::Message;
use protobuf::MessageField;
use protobuf_json_mapping::merge_from_str;
use protobuf_json_mapping::print_to_string;
use protobuf_json_mapping::print_to_string_with_options;
use protobuf_json_mapping::PrintOptions;
use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// The amount of seconds netsimd will wait until the first device has attached.
static IDLE_SECS_FOR_SHUTDOWN: u64 = 15;

const INITIAL_DEVICE_ID: u32 = 1;
const JSON_PRINT_OPTION: PrintOptions = PrintOptions {
    enum_values_int: false,
    proto_field_name: false,
    always_output_default_values: true,
    _future_options: (),
};

/// Logs message on Linux ARM platforms, including thread information.
#[macro_export]
macro_rules! info_linux_arm {
    ($($arg:tt)*) => {
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        {
            let current_thread = std::thread::current();
            let thread_name = current_thread.name().unwrap_or("unnamed");
            let thread_id = current_thread.id();
            log::info!("[Thread: {} ({:?})] {}", thread_name, thread_id, format_args!($($arg)*));
        }
    };
}

lazy_static! {
    static ref DEVICE_MANAGER: Arc<DeviceManager> = Arc::new(DeviceManager::new());
}

fn get_manager() -> Arc<DeviceManager> {
    Arc::clone(&DEVICE_MANAGER)
}

// TODO: last_modified atomic
/// The Device resource is a singleton that manages all devices.
struct DeviceManager {
    // BTreeMap allows ListDevice to output devices in order of identifiers.
    devices: RwLock<BTreeMap<DeviceIdentifier, Device>>,
    ids: AtomicU32,
    last_modified: RwLock<Duration>,
}

impl DeviceManager {
    fn new() -> Self {
        DeviceManager {
            devices: RwLock::new(BTreeMap::new()),
            ids: AtomicU32::new(INITIAL_DEVICE_ID),
            last_modified: RwLock::new(
                SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards"),
            ),
        }
    }

    fn next_id(&self) -> DeviceIdentifier {
        DeviceIdentifier(self.ids.fetch_add(1, Ordering::SeqCst))
    }

    fn update_timestamp(&self) {
        info_linux_arm!("Updated last modified timestamp for devices");
        *self.last_modified.write().unwrap() =
            SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
    }

    /// Get or create a device.
    /// Returns a (device_id, device_name) pair.
    fn get_or_create_device(
        &self,
        guid: Option<&str>,
        name: Option<&str>,
        builtin: bool,
        kind: Option<&str>,
    ) -> (DeviceIdentifier, String) {
        // Hold a lock while checking and updating devices.
        info_linux_arm!("Acquiring write lock on devices");
        let mut guard = self.devices.write().unwrap();
        info_linux_arm!("Acquired write lock");
        // Check if a device with the same guid already exists and if so, return it
        if let Some(guid) = guid {
            if let Some(existing_device) = guard.values().find(|d| d.guid == *guid) {
                if existing_device.builtin != builtin {
                    warn!("builtin mismatch for device {} during add_chip", existing_device.name);
                }
                info_linux_arm!("Releasing write lock");
                return (existing_device.id, existing_device.name.clone());
            }
        }
        // A new device needs to be created and inserted
        let id = self.next_id();
        let default = format!("device-{}", id);
        let name = name.unwrap_or(&default);
        let kind = kind.unwrap_or("UNKNOWN");
        info_linux_arm!("Inserting new device {}", id);
        guard.insert(id, Device::new(id, guid.unwrap_or(&default), name, builtin, kind));
        info_linux_arm!("Releasing write lock");
        drop(guard);
        // Update last modified timestamp for devices
        self.update_timestamp();
        let event = Event::DeviceAdded(DeviceAdded {
            id,
            name: String::from(name),
            builtin,
            kind: String::from(kind),
        });
        info_linux_arm!("Publishing DeviceAdded event: {:?}", event);
        events::publish(event);
        (id, String::from(name))
    }
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
    chip_create_params: &chip::CreateParams,
    wireless_create_params: &wireless::CreateParam,
) -> Result<AddChipResult, String> {
    info_linux_arm!("Adding new chip for device {}", device_guid);
    let chip_kind = chip_create_params.kind;
    let manager = get_manager();
    info_linux_arm!("Getting or creating device {}", device_guid);
    let (device_id, _) = manager.get_or_create_device(
        Some(device_guid),
        Some(device_name),
        chip_kind == ProtoChipKind::BLUETOOTH_BEACON,
        // TODO: add device_kind arg to add_chip and pass to get_or_create_device
        None,
    );
    info_linux_arm!("Device {} retrieved/created: with ID {}", device_guid, device_id);

    // Create
    let chip_id = chip::next_id();
    let wireless_adaptor = wireless::new(wireless_create_params, chip_id);

    // This is infrequent, so we can afford to do another lookup for the device.
    info_linux_arm!("Acquiring write lock on devices");
    let _ = manager
        .devices
        .write()
        .unwrap()
        .get_mut(&device_id)
        .ok_or(format!("Device not found for device_id: {}", device_id))?
        .add_chip(chip_create_params, chip_id, wireless_adaptor);
    info_linux_arm!("Released write lock");

    // Update last modified timestamp for devices
    manager.update_timestamp();

    // Update Capture resource
    let event = Event::ChipAdded(ChipAdded {
        chip_id,
        chip_kind,
        device_name: device_name.to_string(),
        builtin: chip_kind == ProtoChipKind::BLUETOOTH_BEACON,
    });
    info_linux_arm!("Publishing ChipAdded event: {:?}", event);
    events::publish(event);
    Ok(AddChipResult { device_id, chip_id })
}

/// AddChipResult for C++ to handle
pub struct AddChipResultCxx {
    device_id: u32,
    chip_id: u32,
    is_error: bool,
}

impl AddChipResultCxx {
    pub fn get_device_id(&self) -> u32 {
        self.device_id
    }

    pub fn get_chip_id(&self) -> u32 {
        self.chip_id
    }

    pub fn is_error(&self) -> bool {
        self.is_error
    }
}

/// An AddChip function for Rust Device API.
/// The backend gRPC code will be invoking this method.
#[allow(clippy::too_many_arguments)]
pub fn add_chip_cxx(
    device_guid: &str,
    device_name: &str,
    chip_kind: &CxxString,
    chip_address: &str,
    chip_name: &str,
    chip_manufacturer: &str,
    chip_product_name: &str,
    bt_properties: &CxxVector<u8>,
) -> Box<AddChipResultCxx> {
    let bt_properties_proto = Controller::parse_from_bytes(bt_properties.as_slice());
    #[cfg(not(test))]
    let (chip_kind_enum, wireless_create_param) = match chip_kind.to_string().as_str() {
        "BLUETOOTH" => (
            ProtoChipKind::BLUETOOTH,
            wireless::CreateParam::Bluetooth(wireless::bluetooth::CreateParams {
                address: chip_address.to_string(),
                bt_properties: bt_properties_proto
                    .as_ref()
                    .map_or(None, |p| Some(MessageField::some(p.clone()))),
            }),
        ),
        "WIFI" => {
            (ProtoChipKind::WIFI, wireless::CreateParam::Wifi(wireless::wifi::CreateParams {}))
        }
        "UWB" => (
            ProtoChipKind::UWB,
            wireless::CreateParam::Uwb(wireless::uwb::CreateParams {
                address: chip_address.to_string(),
            }),
        ),
        _ => {
            return Box::new(AddChipResultCxx {
                device_id: u32::MAX,
                chip_id: u32::MAX,
                is_error: true,
            })
        }
    };
    #[cfg(test)]
    let (chip_kind_enum, wireless_create_param) = match chip_kind.to_string().as_str() {
        "BLUETOOTH" => (
            ProtoChipKind::BLUETOOTH,
            wireless::CreateParam::Mock(wireless::mocked::CreateParams {
                chip_kind: ProtoChipKind::BLUETOOTH,
            }),
        ),
        "WIFI" => (
            ProtoChipKind::WIFI,
            wireless::CreateParam::Mock(wireless::mocked::CreateParams {
                chip_kind: ProtoChipKind::WIFI,
            }),
        ),
        "UWB" => (
            ProtoChipKind::UWB,
            wireless::CreateParam::Mock(wireless::mocked::CreateParams {
                chip_kind: ProtoChipKind::UWB,
            }),
        ),
        _ => {
            return Box::new(AddChipResultCxx {
                device_id: u32::MAX,
                chip_id: u32::MAX,
                is_error: true,
            })
        }
    };
    let chip_create_params = chip::CreateParams {
        kind: chip_kind_enum,
        address: chip_address.to_string(),
        name: if chip_name.is_empty() { None } else { Some(chip_name.to_string()) },
        manufacturer: chip_manufacturer.to_string(),
        product_name: chip_product_name.to_string(),
        bt_properties: bt_properties_proto.ok(),
    };
    match add_chip(device_guid, device_name, &chip_create_params, &wireless_create_param) {
        Ok(result) => Box::new(AddChipResultCxx {
            device_id: result.device_id.0,
            chip_id: result.chip_id.0,
            is_error: false,
        }),
        Err(_) => {
            Box::new(AddChipResultCxx { device_id: u32::MAX, chip_id: u32::MAX, is_error: true })
        }
    }
}

/// Remove a chip from a device.
///
/// Called when the packet transport for the chip shuts down.
pub fn remove_chip(device_id: DeviceIdentifier, chip_id: ChipIdentifier) -> Result<(), String> {
    let manager = get_manager();
    info_linux_arm!("Acquiring write lock on devices");
    let mut guard = manager.devices.write().unwrap();
    let device =
        guard.get(&device_id).ok_or(format!("RemoveChip device id {device_id} not found"))?;
    let radio_stats = device.remove_chip(&chip_id)?;

    if device.chips.read().unwrap().is_empty() {
        let device = guard
            .remove(&device_id)
            .ok_or(format!("RemoveChip device id {device_id} not found"))?;
        events::publish(Event::DeviceRemoved(DeviceRemoved {
            id: device.id,
            name: device.name,
            builtin: device.builtin,
        }));
    }

    let remaining_nonbuiltin_devices = guard.values().filter(|device| !device.builtin).count();
    info_linux_arm!("Releasing write lock on devices");
    drop(guard);
    events::publish(Event::ChipRemoved(ChipRemoved {
        chip_id,
        device_id,
        remaining_nonbuiltin_devices,
        radio_stats,
    }));

    manager.update_timestamp();
    Ok(())
}

pub fn delete_chip(delete_json: &str) -> Result<(), String> {
    let mut request = DeleteChipRequest::new();
    if merge_from_str(&mut request, delete_json).is_err() {
        return Err(format!(
            "failed to delete chip: incorrectly formatted delete json: {}",
            delete_json
        ));
    };

    let chip_id = ChipIdentifier(request.id);

    info_linux_arm!("Acquiring read lock on devices");
    let device_id = get_manager()
        .devices
        .read()
        .unwrap()
        .iter()
        .find(|(_, device)| device.chips.read().unwrap().contains_key(&chip_id))
        .map(|(id, _)| *id)
        .ok_or(format!("failed to delete chip: could not find chip with id {}", request.id))?;
    info_linux_arm!("Released read lock");

    remove_chip(device_id, chip_id)
}

/// A RemoveChip function for Rust Device API.
/// The backend gRPC code will be invoking this method.
pub fn remove_chip_cxx(device_id: u32, chip_id: u32) {
    let _ = remove_chip(DeviceIdentifier(device_id), ChipIdentifier(chip_id));
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
    let manager = get_manager();
    // Check if specified device name is already mapped.
    info_linux_arm!("Acquiring read lock on devices");
    if new_device.name != String::default()
        && manager.devices.read().unwrap().values().any(|d| d.guid == new_device.name)
    {
        info_linux_arm!("Released read lock");
        return Err(String::from("failed to create device: device already exists"));
    }
    info_linux_arm!("Released read lock");

    if new_device.chips.is_empty() {
        return Err(String::from("failed to create device: device must contain at least 1 chip"));
    }
    new_device.chips.iter().try_for_each(|chip| match chip.chip {
        Some(ProtoBuiltin::BleBeacon(_)) => Ok(()),
        Some(_) => Err(format!("failed to create device: chip {} was not a built-in", chip.name)),
        None => Err(format!("failed to create device: chip {} was missing a radio", chip.name)),
    })?;

    let device_name = (new_device.name != String::default()).then_some(new_device.name.as_str());
    let (device_id, device_name) =
        manager.get_or_create_device(device_name, device_name, true, Some("BluetoothBeacon"));

    new_device.chips.iter().try_for_each(|chip| {
        {
            let chip_create_params = chip::CreateParams {
                kind: chip.kind.enum_value_or_default(),
                address: chip.address.clone(),
                name: if chip.name.is_empty() { None } else { Some(chip.name.to_string()) },
                manufacturer: chip.manufacturer.clone(),
                product_name: chip.product_name.clone(),
                bt_properties: chip.bt_properties.as_ref().cloned(),
            };
            let wireless_create_params =
                wireless::CreateParam::BleBeacon(wireless::ble_beacon::CreateParams {
                    device_name: device_name.clone(),
                    chip_proto: chip.clone(),
                });
            add_chip(&device_name, &device_name, &chip_create_params, &wireless_create_params)
        }
        .map(|_| ())
    })?;

    Ok(device_id)
}

// lock the devices, find the id and call the patch function
pub fn patch_device(
    id_option: Option<DeviceIdentifier>,
    patch_device_request: PatchDeviceRequest,
) -> Result<(), String> {
    let manager = get_manager();
    let proto_device = patch_device_request.device;
    match id_option {
        Some(id) => match manager.devices.read().unwrap().get(&id) {
            Some(device) => {
                let result = device.patch(&proto_device);
                let name = device.name.clone();
                if result.is_ok() {
                    // Update last modified timestamp for manager
                    manager.update_timestamp();

                    // Publish Device Patched event
                    events::publish(Event::DevicePatched(DevicePatched { id, name }));
                }
                result
            }
            None => Err(format!("No such device with id {id}")),
        },
        None => {
            let mut multiple_matches = false;
            let mut target: Option<&Device> = None;
            let devices = manager.devices.read().unwrap();
            for device in devices.values() {
                if device.name.contains(&proto_device.name) {
                    if device.name == proto_device.name {
                        let result = device.patch(&proto_device);
                        let id = device.id;
                        let name = device.name.clone();
                        if result.is_ok() {
                            // Update last modified timestamp for manager
                            manager.update_timestamp();

                            // Publish Device Patched event
                            events::publish(Event::DevicePatched(DevicePatched { id, name }));
                        }
                        return result;
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
                    let id = device.id;
                    let name = device.name.clone();
                    if result.is_ok() {
                        // Update last modified timestamp for devices
                        manager.update_timestamp();

                        // Publish Device Patched event
                        events::publish(Event::DevicePatched(DevicePatched { id, name }));
                    }
                    result
                }
                None => Err(format!("No such device with name {}", proto_device.name)),
            }
        }
    }
}

// Parse from input json string to proto
#[allow(dead_code)]
fn patch_device_json(id_option: Option<DeviceIdentifier>, patch_json: &str) -> Result<(), String> {
    let mut patch_device_request = PatchDeviceRequest::new();
    if merge_from_str(&mut patch_device_request, patch_json).is_ok() {
        patch_device(id_option, patch_device_request)
    } else {
        Err(format!("Incorrect format of patch json {}", patch_json))
    }
}

fn distance(a: &ProtoPosition, b: &ProtoPosition) -> f32 {
    ((b.x - a.x).powf(2.0) + (b.y - a.y).powf(2.0) + (b.z - a.z).powf(2.0)).sqrt()
}

#[allow(dead_code)]
fn get_distance(id: &ChipIdentifier, other_id: &ChipIdentifier) -> Result<f32, String> {
    let device_id = crate::devices::chip::get_chip(id)
        .ok_or(format!("No such device with chip_id {id}"))?
        .device_id;
    let other_device_id = crate::devices::chip::get_chip(other_id)
        .ok_or(format!("No such device with chip_id {other_id}"))?
        .device_id;
    let manager = get_manager();
    info_linux_arm!("Acquiring read lock on devices");
    let a = manager
        .devices
        .read()
        .unwrap()
        .get(&device_id)
        .map(|device_ref| device_ref.position.read().unwrap().clone())
        .ok_or(format!("No such device with id {id}"))?;
    info_linux_arm!("Released read lock");
    info_linux_arm!("Acquiring read lock on devices");
    let b = manager
        .devices
        .read()
        .unwrap()
        .get(&other_device_id)
        .map(|device_ref| device_ref.position.read().unwrap().clone())
        .ok_or(format!("No such device with id {other_id}"))?;
    info_linux_arm!("Released read lock");
    Ok(distance(&a, &b))
}

/// A GetDistance function for Rust Device API.
/// The backend gRPC code will be invoking this method.
pub fn get_distance_cxx(a: u32, b: u32) -> f32 {
    match get_distance(&ChipIdentifier(a), &ChipIdentifier(b)) {
        Ok(distance) => distance,
        Err(err) => {
            warn!("get_distance Error: {err}");
            0.0
        }
    }
}

/// Function to obtain ProtoDevice given a ChipIdentifier
pub fn get_device(chip_id: &ChipIdentifier) -> anyhow::Result<netsim_proto::model::Device> {
    let device_id = match chip::get_chip(chip_id) {
        Some(chip) => chip.device_id,
        None => return Err(anyhow::anyhow!("Can't find chip for chip_id: {chip_id}")),
    };
    info_linux_arm!("Acquiring read lock on devices");
    let manager = get_manager();
    let guard = manager.devices.read().unwrap();
    let res = guard
        .get(&device_id)
        .ok_or(anyhow::anyhow!("Can't find device for device_id: {device_id}"))?
        .get()
        .map_err(|e| anyhow::anyhow!("{e:?}"));
    drop(guard);
    info_linux_arm!("Released read lock");
    res
}

pub fn reset_all() -> Result<(), String> {
    let manager = get_manager();
    // Perform reset for all manager
    info_linux_arm!("Acquiring read lock on devices");
    for device in manager.devices.read().unwrap().values() {
        device.reset()?;
    }
    info_linux_arm!("Released read lock");
    // Update last modified timestamp for manager
    manager.update_timestamp();
    events::publish(Event::DeviceReset);
    Ok(())
}

fn handle_device_create(writer: ResponseWritable, create_json: &str) {
    let mut response = CreateDeviceResponse::new();

    let mut collate_results = || {
        let id = create_device(create_json)?;

        info_linux_arm!("Acquiring read lock on devices");
        let device_proto = get_manager()
            .devices
            .read()
            .unwrap()
            .get(&id)
            .ok_or("failed to create device")?
            .get()?;
        info_linux_arm!("Released read lock");
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
    match patch_device_json(id, patch_json) {
        Ok(()) => writer.put_ok("text/plain", "Device Patch Success", vec![]),
        Err(err) => writer.put_error(404, err.as_str()),
    }
}

fn handle_chip_delete(writer: ResponseWritable, delete_json: &str) {
    match delete_chip(delete_json) {
        Ok(()) => writer.put_ok("text/plain", "Chip Delete Success", vec![]),
        Err(err) => writer.put_error(404, err.as_str()),
    }
}

pub fn list_device() -> anyhow::Result<ListDeviceResponse, String> {
    // Instantiate ListDeviceResponse and add DeviceManager
    let mut response = ListDeviceResponse::new();
    let manager = get_manager();

    info_linux_arm!("Acquiring read lock on devices");
    for device in manager.devices.read().unwrap().values() {
        if let Ok(device_proto) = device.get() {
            response.devices.push(device_proto);
        }
    }
    info_linux_arm!("Released read lock");

    // Add Last Modified Timestamp into ListDeviceResponse
    response.last_modified = Some(Timestamp {
        seconds: manager.last_modified.read().unwrap().as_secs() as i64,
        nanos: manager.last_modified.read().unwrap().subsec_nanos() as i32,
        ..Default::default()
    })
    .into();
    Ok(response)
}

/// Performs ListDevices to get the list of DeviceManager and write to writer.
fn handle_device_list(writer: ResponseWritable) {
    let response = list_device().unwrap();
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

/// Performs SubscribeDevice
fn handle_device_subscribe(writer: ResponseWritable, subscribe_json: &str) {
    // Check if the provided last_modified timestamp is prior to the current last_modified
    let mut subscribe_device_request = SubscribeDeviceRequest::new();
    if merge_from_str(&mut subscribe_device_request, subscribe_json).is_ok() {
        let timestamp_proto = subscribe_device_request.last_modified;
        let provided_last_modified =
            Duration::new(timestamp_proto.seconds as u64, timestamp_proto.nanos as u32);
        if provided_last_modified < *get_manager().last_modified.read().unwrap() {
            info!("Immediate return for SubscribeDevice");
            handle_device_list(writer);
            return;
        }
    }

    let event_rx = events::subscribe();
    // Timeout after 15 seconds with no event received
    match event_rx.recv_timeout(Duration::from_secs(15)) {
        Ok(Event::DeviceAdded(_))
        | Ok(Event::DeviceRemoved(_))
        | Ok(Event::ChipAdded(_))
        | Ok(Event::ChipRemoved(_))
        | Ok(Event::DevicePatched(_))
        | Ok(Event::DeviceReset) => handle_device_list(writer),
        Err(err) => writer.put_error(404, format!("{err:?}").as_str()),
        _ => writer.put_error(404, "disconnecting due to unrelated event"),
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
            "SUBSCRIBE" => {
                let body = request.body();
                let subscribe_json = String::from_utf8(body.to_vec()).unwrap();
                handle_device_subscribe(writer, subscribe_json.as_str());
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
            "DELETE" => {
                let body = &request.body();
                let delete_json = String::from_utf8(body.to_vec()).unwrap();
                handle_chip_delete(writer, delete_json.as_str());
            }
            _ => writer.put_error(404, "Not found."),
        }
    } else {
        // Routes with ID specified
        match request.method().as_str() {
            "PATCH" => {
                let id = match param.parse::<u32>() {
                    Ok(num) => DeviceIdentifier(num),
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

/// return enum type for check_device_event
#[derive(Debug, PartialEq)]
enum DeviceWaitStatus {
    LastDeviceRemoved,
    DeviceAdded,
    Timeout,
    IgnoreEvent,
}

/// listening to events
fn check_device_event(
    events_rx: &Receiver<Event>,
    timeout_time: Option<Instant>,
) -> DeviceWaitStatus {
    let wait_time = timeout_time.map_or(Duration::from_secs(u64::MAX), |t| t - Instant::now());
    match events_rx.recv_timeout(wait_time) {
        Ok(Event::ChipRemoved(ChipRemoved { remaining_nonbuiltin_devices: 0, .. })) => {
            DeviceWaitStatus::LastDeviceRemoved
        }
        // DeviceAdded (event from CreateDevice)
        // ChipAdded (event from add_chip or add_chip_cxx)
        Ok(Event::DeviceAdded(DeviceAdded { builtin: false, .. }))
        | Ok(Event::ChipAdded(ChipAdded { builtin: false, .. })) => DeviceWaitStatus::DeviceAdded,
        Err(_) => DeviceWaitStatus::Timeout,
        _ => DeviceWaitStatus::IgnoreEvent,
    }
}

/// wait loop logic for devices
/// the function will publish a ShutDown event when
/// 1. Initial timeout before first device is added
/// 2. Last Chip Removed from netsimd
/// this function should NOT be invoked if running in no-shutdown mode
pub fn spawn_shutdown_publisher(events_rx: Receiver<Event>) {
    spawn_shutdown_publisher_with_timeout(events_rx, IDLE_SECS_FOR_SHUTDOWN, events::get_events());
}

// separate function for testability
fn spawn_shutdown_publisher_with_timeout(
    events_rx: Receiver<Event>,
    timeout_duration_s: u64,
    events_tx: Arc<Mutex<Events>>,
) {
    let _ =
        std::thread::Builder::new().name("device_event_subscriber".to_string()).spawn(move || {
            let publish_event =
                |e: Event| events_tx.lock().expect("Failed to acquire lock on events").publish(e);

            let mut timeout_time = Some(Instant::now() + Duration::from_secs(timeout_duration_s));
            loop {
                match check_device_event(&events_rx, timeout_time) {
                    DeviceWaitStatus::LastDeviceRemoved => {
                        publish_event(Event::ShutDown(ShutDown {
                            reason: "last device disconnected".to_string(),
                        }));
                        return;
                    }
                    DeviceWaitStatus::DeviceAdded => {
                        timeout_time = None;
                    }
                    DeviceWaitStatus::Timeout => {
                        publish_event(Event::ShutDown(ShutDown {
                            reason: format!(
                                "no devices connected within {IDLE_SECS_FOR_SHUTDOWN}s"
                            ),
                        }));
                        return;
                    }
                    DeviceWaitStatus::IgnoreEvent => continue,
                }
            }
        });
}

/// Return vector containing current radio chip stats from all devices
pub fn get_radio_stats() -> Vec<NetsimRadioStats> {
    let mut result: Vec<NetsimRadioStats> = Vec::new();
    // TODO: b/309805437 - optimize logic using get_stats for WirelessAdaptor
    info_linux_arm!("Acquiring read lock on devices");
    for (device_id, device) in get_manager().devices.read().unwrap().iter() {
        for chip in device.chips.read().unwrap().values() {
            for mut radio_stats in chip.get_stats() {
                info_linux_arm!("Got status for device {} on chip {}", device_id, chip.id);
                radio_stats.set_device_id(device_id.0);
                result.push(radio_stats);
            }
        }
    }
    info_linux_arm!("Released read lock");
    result
}

#[cfg(test)]
mod tests {
    use crate::events;
    use netsim_common::util::netsim_logger::init_for_test;
    use netsim_proto::model::{
        Device as ProtoDevice, DeviceCreate as ProtoDeviceCreate, Orientation as ProtoOrientation,
    };
    use protobuf_json_mapping::print_to_string;
    use std::{sync::Once, thread};

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
        kind: String,
    }

    impl TestChipParameters {
        fn add_chip(&self) -> Result<AddChipResult, String> {
            let chip_create_params = chip::CreateParams {
                kind: self.chip_kind,
                address: "".to_string(),
                name: Some(self.chip_name.clone()),
                manufacturer: self.chip_manufacturer.clone(),
                product_name: self.chip_product_name.clone(),
                bt_properties: None,
            };
            let wireless_create_params =
                wireless::CreateParam::Mock(wireless::mocked::CreateParams {
                    chip_kind: self.chip_kind,
                });
            super::add_chip(
                &self.device_guid,
                &self.device_name,
                &chip_create_params,
                &wireless_create_params,
            )
        }

        fn get_or_create_device(&self) -> DeviceIdentifier {
            let manager = get_manager();
            manager
                .get_or_create_device(
                    Some(&self.device_guid),
                    Some(&self.device_name),
                    false,
                    Some(&self.kind),
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

    fn test_chip_1_bt() -> TestChipParameters {
        TestChipParameters {
            device_guid: format!("guid-fs-1-{:?}", thread::current().id()),
            device_name: format!("test-device-name-1-{:?}", thread::current().id()),
            chip_kind: ProtoChipKind::BLUETOOTH,
            chip_name: "bt_chip_name".to_string(),
            chip_manufacturer: "netsim".to_string(),
            chip_product_name: "netsim_bt".to_string(),
            kind: "TESTDEVICE".to_string(),
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
            kind: "TESTDEVICE".to_string(),
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
            kind: "TESTDEVICE".to_string(),
        }
    }

    fn reset(id: DeviceIdentifier) -> Result<(), String> {
        let manager = get_manager();
        info_linux_arm!("Acquiring write lock on devices");
        let mut devices = manager.devices.write().unwrap();
        let res = match devices.get_mut(&id) {
            Some(device) => device.reset(),
            None => Err(format!("No such device with id {id}")),
        };
        info_linux_arm!("Released write lock");
        res
    }

    fn spawn_shutdown_publisher_test_setup(timeout: u64) -> (Arc<Mutex<Events>>, Receiver<Event>) {
        let mut events = events::test::new();
        let events_rx = events::test::subscribe(&mut events);
        spawn_shutdown_publisher_with_timeout(events_rx, timeout, events.clone());

        let events_rx2 = events::test::subscribe(&mut events);

        (events, events_rx2)
    }

    #[test]
    fn test_spawn_shutdown_publisher_last_chip_removed() {
        let (mut events, events_rx) = spawn_shutdown_publisher_test_setup(IDLE_SECS_FOR_SHUTDOWN);

        events::test::publish(
            &mut events,
            Event::ChipRemoved(ChipRemoved {
                remaining_nonbuiltin_devices: 0,
                ..Default::default()
            }),
        );

        // receive our own ChipRemoved
        assert!(matches!(events_rx.recv(), Ok(Event::ChipRemoved(ChipRemoved { .. }))));
        // receive the ShutDown emitted by the function under test
        assert!(matches!(events_rx.recv(), Ok(Event::ShutDown(ShutDown { .. }))));
    }

    #[test]
    fn test_spawn_shutdown_publisher_chip_removed_which_is_not_last_chip() {
        let (mut events, events_rx) = spawn_shutdown_publisher_test_setup(IDLE_SECS_FOR_SHUTDOWN);
        events::test::publish(
            &mut events,
            Event::ChipRemoved(ChipRemoved {
                chip_id: ChipIdentifier(1),
                remaining_nonbuiltin_devices: 1,
                ..Default::default()
            }),
        );

        // give other thread time to generate a ShutDown if it was going to
        std::thread::sleep(std::time::Duration::from_secs(1));

        // only the 2nd ChipRemoved should generate a ShutDown as it is marked the last one
        events::test::publish(
            &mut events,
            Event::ChipRemoved(ChipRemoved {
                chip_id: ChipIdentifier(0),
                remaining_nonbuiltin_devices: 0,
                ..Default::default()
            }),
        );

        // receive our own ChipRemoved
        assert!(matches!(events_rx.recv(), Ok(Event::ChipRemoved(ChipRemoved { .. }))));
        // receive our own ChipRemoved (with no shutdown)
        assert!(matches!(events_rx.recv(), Ok(Event::ChipRemoved(ChipRemoved { .. }))));
        // only then receive the ShutDown emitted by the function under test
        assert!(matches!(events_rx.recv(), Ok(Event::ShutDown(ShutDown { .. }))));
    }

    #[test]
    fn test_spawn_shutdown_publisher_last_chip_removed_with_duplicate_event() {
        let (mut events, events_rx) = spawn_shutdown_publisher_test_setup(IDLE_SECS_FOR_SHUTDOWN);
        events::test::publish(
            &mut events,
            Event::ChipRemoved(ChipRemoved {
                chip_id: ChipIdentifier(0),
                remaining_nonbuiltin_devices: 0,
                ..Default::default()
            }),
        );

        // give other thread time to generate a ShutDown if it was going to
        std::thread::sleep(std::time::Duration::from_secs(1));

        // this is a duplicate event and we already sent that all chips were removed
        // this is for strict comparison with test_spawn_shutdown_publisher_chip_removed_which_is_not_last_chip
        // to validate that if the first event has remaining_nonbuiltin_devices 0
        // we would receive ChipRemoved, ShutDown, ChipRemoved
        // but if first ChipRemoved has remaining_nonbuiltin_devices,
        // we instead receive ChipRemoved, ChipRemoved, ShutDown
        events::test::publish(
            &mut events,
            Event::ChipRemoved(ChipRemoved {
                chip_id: ChipIdentifier(0),
                remaining_nonbuiltin_devices: 0,
                ..Default::default()
            }),
        );

        // receive our own ChipRemoved
        assert!(matches!(events_rx.recv(), Ok(Event::ChipRemoved(_))));
        // receive the ShutDown emitted by the function under test
        assert!(matches!(events_rx.recv(), Ok(Event::ShutDown(_))));
        // receive our own erroneous ChipRemoved which occurs after we said all chips were removed
        // this is just for strict comparison with test_spawn_shutdown_publisher_chip_removed_which_is_not_last_chip
        assert!(matches!(events_rx.recv(), Ok(Event::ChipRemoved(_))));
        // should timeout now (no further events as we expect shutdown publisher thread to have stopped)
        assert!(events_rx.recv_timeout(Duration::from_secs(2)).is_err());
    }

    #[test]
    fn test_spawn_shutdown_publisher_timeout() {
        let (_, events_rx) = spawn_shutdown_publisher_test_setup(1u64);

        // receive the ShutDown emitted by the function under test
        assert!(matches!(events_rx.recv_timeout(Duration::from_secs(2)), Ok(Event::ShutDown(_))));
    }

    #[test]
    fn test_spawn_shutdown_publisher_timeout_is_canceled_if_a_chip_is_added() {
        let (mut events, events_rx) = spawn_shutdown_publisher_test_setup(1u64);

        events::test::publish(
            &mut events,
            Event::ChipAdded(ChipAdded {
                chip_id: ChipIdentifier(0),
                chip_kind: ProtoChipKind::BLUETOOTH,
                ..Default::default()
            }),
        );
        assert!(matches!(events_rx.recv(), Ok(Event::ChipAdded(_))));

        // should NO longer receive the ShutDown emitted by the function under test
        // based on timeout removed when chip added
        assert!(events_rx.recv_timeout(Duration::from_secs(2)).is_err());

        events::test::publish(
            &mut events,
            Event::ChipRemoved(ChipRemoved {
                chip_id: ChipIdentifier(0),
                remaining_nonbuiltin_devices: 0,
                ..Default::default()
            }),
        );
        // receive our own ChipRemoved
        assert!(matches!(events_rx.recv(), Ok(Event::ChipRemoved(_))));
        // receive the ShutDown emitted by the function under test
        assert!(matches!(events_rx.recv(), Ok(Event::ShutDown(_))));
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
        match get_manager().devices.read().unwrap().get(&chip_result.device_id) {
            Some(device) => {
                let chips = device.chips.read().unwrap();
                let chip = chips.get(&chip_result.chip_id).unwrap();
                assert_eq!(chip_params.chip_kind, chip.kind);
                assert_eq!(
                    chip_params.chip_manufacturer,
                    chip.manufacturer.read().unwrap().to_string()
                );
                assert_eq!(chip_params.chip_name, chip.name);
                assert_eq!(
                    chip_params.chip_product_name,
                    chip.product_name.read().unwrap().to_string()
                );
                assert_eq!(chip_params.device_name, device.name);
            }
            None => unreachable!(),
        }
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
    fn test_patch_device_json() {
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
        proto_device.visible = Some(false);
        proto_device.position = Some(request_position.clone()).into();
        proto_device.orientation = Some(request_orientation.clone()).into();
        patch_device_request.device = Some(proto_device.clone()).into();
        let patch_json = print_to_string(&patch_device_request).unwrap();
        patch_device_json(Some(chip_result.device_id), patch_json.as_str()).unwrap();
        match get_manager().devices.read().unwrap().get(&chip_result.device_id) {
            Some(device) => {
                assert_eq!(device.position.read().unwrap().x, request_position.x);
                assert_eq!(device.position.read().unwrap().y, request_position.y);
                assert_eq!(device.position.read().unwrap().z, request_position.z);
                assert_eq!(device.orientation.read().unwrap().yaw, request_orientation.yaw);
                assert_eq!(device.orientation.read().unwrap().pitch, request_orientation.pitch);
                assert_eq!(device.orientation.read().unwrap().roll, request_orientation.roll);
                assert!(!device.visible.load(Ordering::SeqCst));
            }
            None => unreachable!(),
        }

        // Patch device by name with substring match
        proto_device.name = format!("test-device-name-1-{:?}", thread::current().id());
        patch_device_request.device = Some(proto_device).into();
        let patch_json = print_to_string(&patch_device_request).unwrap();
        assert!(patch_device_json(None, patch_json.as_str()).is_ok());
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
        let patch_result = patch_device_json(Some(bt_chip_result.device_id), error_json.as_str());
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
        let patch_result = patch_device_json(Some(bt_chip_result.device_id), error_json.as_str());
        assert!(patch_result.is_err());
        assert_eq!(
            patch_result.unwrap_err(),
            format!("Incorrect format of patch json {}", error_json)
        );

        // Incorrect Id
        let error_json = r#"{"device": {"name": "test-device-name-1"}}"#;
        let patch_result =
            patch_device_json(Some(DeviceIdentifier(INITIAL_DEVICE_ID - 1)), error_json);
        assert!(patch_result.is_err());
        assert_eq!(
            patch_result.unwrap_err(),
            format!("No such device with id {}", INITIAL_DEVICE_ID - 1)
        );

        // Incorrect name
        let error_json = r#"{"device": {"name": "wrong-name"}}"#;
        let patch_result = patch_device_json(None, error_json);
        assert!(patch_result.is_err());
        assert_eq!(patch_result.unwrap_err(), "No such device with name wrong-name");

        // Multiple ambiguous matching
        let error_json = r#"{"device": {"name": "test-device"}}"#;
        let patch_result = patch_device_json(None, error_json);
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
        let manager = get_manager();
        let devices = manager.devices.read().unwrap();
        let device = devices.get(&bt_chip_result.device_id).unwrap();
        assert_eq!(device.id, bt_chip_result.device_id);
        assert_eq!(device.name, bt_chip_params.device_name);
        assert_eq!(device.chips.read().unwrap().len(), 2);
        for chip in device.chips.read().unwrap().values() {
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
        proto_device.visible = Some(false);
        proto_device.position = Some(request_position).into();
        proto_device.orientation = Some(request_orientation).into();
        patch_device_request.device = Some(proto_device).into();
        patch_device_json(
            Some(chip_result.device_id),
            print_to_string(&patch_device_request).unwrap().as_str(),
        )
        .unwrap();
        match get_manager().devices.read().unwrap().get(&chip_result.device_id) {
            Some(device) => {
                assert_eq!(device.position.read().unwrap().x, 10.0);
                assert_eq!(device.orientation.read().unwrap().yaw, 1.0);
                assert!(!device.visible.load(Ordering::SeqCst));
            }
            None => unreachable!(),
        }
        reset(chip_result.device_id).unwrap();
        match get_manager().devices.read().unwrap().get(&chip_result.device_id) {
            Some(device) => {
                assert_eq!(device.position.read().unwrap().x, 0.0);
                assert_eq!(device.position.read().unwrap().y, 0.0);
                assert_eq!(device.position.read().unwrap().z, 0.0);
                assert_eq!(device.orientation.read().unwrap().yaw, 0.0);
                assert_eq!(device.orientation.read().unwrap().pitch, 0.0);
                assert_eq!(device.orientation.read().unwrap().roll, 0.0);
                assert!(device.visible.load(Ordering::SeqCst));
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
        match get_manager().devices.read().unwrap().get(&bt_chip_result.device_id) {
            Some(device) => {
                assert_eq!(device.chips.read().unwrap().len(), 1);
                assert_eq!(
                    device.chips.read().unwrap().get(&wifi_chip_result.chip_id).unwrap().kind,
                    ProtoChipKind::WIFI
                );
            }
            None => unreachable!(),
        }

        // Remove a wifi chip of first device
        remove_chip(wifi_chip_result.device_id, wifi_chip_result.chip_id).unwrap();
        assert!(!get_manager().devices.read().unwrap().contains_key(&wifi_chip_result.device_id));

        // Remove a bt chip of second device
        remove_chip(bt_chip_2_result.device_id, bt_chip_2_result.chip_id).unwrap();
        assert!(!get_manager().devices.read().unwrap().contains_key(&bt_chip_2_result.device_id));
    }

    #[test]
    fn test_remove_chip_error() {
        // Initializing Logger
        logger_setup();

        // Add 2 chips of same device and 1 chip of different device
        let bt_chip_params = test_chip_1_bt();
        let bt_chip_result = bt_chip_params.add_chip().unwrap();

        // Invoke remove_chip with incorrect chip_id.
        match remove_chip(bt_chip_result.device_id, ChipIdentifier(9999)) {
            Ok(_) => unreachable!(),
            Err(err) => assert_eq!(err, "RemoveChip chip id 9999 not found"),
        }

        // Invoke remove_chip with incorrect device_id
        match remove_chip(DeviceIdentifier(9999), bt_chip_result.chip_id) {
            Ok(_) => unreachable!(),
            Err(err) => assert_eq!(err, "RemoveChip device id 9999 not found"),
        }
        assert!(get_manager().devices.read().unwrap().contains_key(&bt_chip_result.device_id));
    }

    #[test]
    fn test_get_distance() {
        // Initializing Logger
        logger_setup();

        // Add 2 chips of different devices
        let bt_chip_params = test_chip_1_bt();
        let bt_chip_2_params = test_chip_2_bt();
        let bt_chip_result = bt_chip_params.add_chip().unwrap();
        let bt_chip_2_result = bt_chip_2_params.add_chip().unwrap();

        // Patch the first chip
        let mut patch_device_request = PatchDeviceRequest::new();
        let mut proto_device = ProtoDevice::new();
        let request_position = new_position(1.0, 1.0, 1.0);
        proto_device.name = bt_chip_params.device_name;
        proto_device.position = Some(request_position.clone()).into();
        patch_device_request.device = Some(proto_device.clone()).into();
        let patch_json = print_to_string(&patch_device_request).unwrap();
        patch_device_json(Some(bt_chip_result.device_id), patch_json.as_str()).unwrap();

        // Patch the second chip
        let mut patch_device_request = PatchDeviceRequest::new();
        let mut proto_device = ProtoDevice::new();
        let request_position = new_position(1.0, 4.0, 5.0);
        proto_device.name = bt_chip_2_params.device_name;
        proto_device.position = Some(request_position.clone()).into();
        patch_device_request.device = Some(proto_device.clone()).into();
        let patch_json = print_to_string(&patch_device_request).unwrap();
        patch_device_json(Some(bt_chip_2_result.device_id), patch_json.as_str()).unwrap();

        // Verify the get_distance performs the correct computation of
        // sqrt((1-1)**2 + (4-1)**2 + (5-1)**2)
        assert_eq!(Ok(5.0), get_distance(&bt_chip_result.chip_id, &bt_chip_2_result.chip_id))
    }

    #[allow(dead_code)]
    fn list_request() -> Request<Vec<u8>> {
        Request::builder()
            .method("GET")
            .uri("/v1/devices")
            .version(Version::HTTP_11)
            .body(Vec::<u8>::new())
            .unwrap()
    }

    use netsim_proto::model::chip::{
        ble_beacon::AdvertiseData, ble_beacon::AdvertiseSettings, BleBeacon, Chip,
    };
    use netsim_proto::model::chip_create::{BleBeaconCreate, Chip as BuiltChipProto};
    use netsim_proto::model::Chip as ChipProto;
    use netsim_proto::model::ChipCreate as ProtoChipCreate;
    use netsim_proto::model::Device as DeviceProto;
    use protobuf::{EnumOrUnknown, MessageField};

    fn get_test_create_device_request(device_name: Option<String>) -> CreateDeviceRequest {
        let beacon_proto = BleBeaconCreate {
            settings: MessageField::some(AdvertiseSettings { ..Default::default() }),
            adv_data: MessageField::some(AdvertiseData { ..Default::default() }),
            ..Default::default()
        };

        let chip_proto = ProtoChipCreate {
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
        let manager = get_manager();
        let devices = manager.devices.read().unwrap();
        let device = devices.get(&id).expect("could not find test bluetooth beacon device");

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
                chips: vec![ProtoChipCreate::default()],
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

        let manager = get_manager();
        let mut devices = manager.devices.write().unwrap();

        let device = devices.get_mut(&id).expect("could not find test bluetooth beacon device");

        let device_proto = device.get();
        assert!(device_proto.is_ok(), "{}", device_proto.unwrap_err());
        let device_proto = device_proto.unwrap();

        let patch_result = device.patch(&DeviceProto {
            name: device_proto.name.clone(),
            id: id.0,
            chips: vec![ChipProto {
                name: request.device.chips[0].name.clone(),
                kind: EnumOrUnknown::new(ProtoChipKind::BLUETOOTH_BEACON),
                chip: Some(Chip::BleBeacon(BleBeacon {
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

    #[test]
    fn test_remove_beacon_device_succeeds() {
        logger_setup();

        let create_request = get_test_create_device_request(None);
        let device_id = create_device(&print_to_string(&create_request).unwrap());
        assert!(device_id.is_ok(), "{}", device_id.unwrap_err());

        let device_id = device_id.unwrap();
        let chip_id = {
            let manager = get_manager();
            let devices = manager.devices.read().unwrap();
            let device = devices.get(&device_id).unwrap();
            let chips = device.chips.read().unwrap();
            chips.first_key_value().map(|(id, _)| *id).unwrap()
        };

        let delete_request = DeleteChipRequest { id: chip_id.0, ..Default::default() };
        let delete_result = delete_chip(&print_to_string(&delete_request).unwrap());
        assert!(delete_result.is_ok(), "{}", delete_result.unwrap_err());

        assert!(!get_manager().devices.read().unwrap().contains_key(&device_id))
    }

    #[test]
    fn test_remove_beacon_device_fails() {
        logger_setup();

        let create_request = get_test_create_device_request(None);
        let device_id = create_device(&print_to_string(&create_request).unwrap());
        assert!(device_id.is_ok(), "{}", device_id.unwrap_err());

        let device_id = device_id.unwrap();
        let chip_id = get_manager()
            .devices
            .read()
            .unwrap()
            .get(&device_id)
            .unwrap()
            .chips
            .read()
            .unwrap()
            .first_key_value()
            .map(|(id, _)| *id)
            .unwrap();

        let delete_request = DeleteChipRequest { id: chip_id.0, ..Default::default() };
        let delete_result = delete_chip(&print_to_string(&delete_request).unwrap());
        assert!(delete_result.is_ok(), "{}", delete_result.unwrap_err());

        let delete_result = delete_chip(&print_to_string(&delete_request).unwrap());
        assert!(delete_result.is_err());
    }

    #[test]
    fn test_check_device_event_initial_timeout() {
        logger_setup();

        let mut events = events::test::new();
        let events_rx = events::test::subscribe(&mut events);
        assert_eq!(
            check_device_event(&events_rx, Some(std::time::Instant::now())),
            DeviceWaitStatus::Timeout
        );
    }

    #[test]
    fn test_check_device_event_last_device_removed() {
        logger_setup();

        let mut events = events::test::new();
        let events_rx = events::test::subscribe(&mut events);
        events::test::publish(
            &mut events,
            Event::ChipRemoved(ChipRemoved {
                remaining_nonbuiltin_devices: 0,
                ..Default::default()
            }),
        );
        assert_eq!(check_device_event(&events_rx, None), DeviceWaitStatus::LastDeviceRemoved);
    }

    #[test]
    fn test_check_device_event_device_chip_added() {
        logger_setup();

        let mut events = events::test::new();
        let events_rx = events::test::subscribe(&mut events);
        events::test::publish(
            &mut events,
            Event::DeviceAdded(DeviceAdded {
                id: DeviceIdentifier(0),
                name: "".to_string(),
                builtin: false,
                kind: "TestDevice".to_string(),
            }),
        );
        assert_eq!(check_device_event(&events_rx, None), DeviceWaitStatus::DeviceAdded);
        events::test::publish(
            &mut events,
            Event::ChipAdded(ChipAdded { builtin: false, ..Default::default() }),
        );
        assert_eq!(check_device_event(&events_rx, None), DeviceWaitStatus::DeviceAdded);
    }

    #[test]
    fn test_check_device_event_ignore_event() {
        logger_setup();

        let mut events = events::test::new();
        let events_rx = events::test::subscribe(&mut events);
        events::test::publish(
            &mut events,
            Event::DevicePatched(DevicePatched { id: DeviceIdentifier(0), name: "".to_string() }),
        );
        assert_eq!(check_device_event(&events_rx, None), DeviceWaitStatus::IgnoreEvent);
        events::test::publish(
            &mut events,
            Event::ChipRemoved(ChipRemoved {
                remaining_nonbuiltin_devices: 1,
                ..Default::default()
            }),
        );
        assert_eq!(check_device_event(&events_rx, None), DeviceWaitStatus::IgnoreEvent);
    }

    #[test]
    fn test_check_device_event_ignore_chip_added_for_builtin() {
        logger_setup();

        let mut events = events::test::new();
        let events_rx = events::test::subscribe(&mut events);
        events::test::publish(
            &mut events,
            Event::ChipAdded(ChipAdded { builtin: true, ..Default::default() }),
        );
        assert_eq!(check_device_event(&events_rx, None), DeviceWaitStatus::IgnoreEvent);
    }
}
