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

use frontend_proto::model::chip::Bluetooth as ProtoBluetoothChip;
use frontend_proto::model::chip::Radio as ProtoRadioChip;

use super::device::DeviceIdentifier;

pub type FacadeIdentifier = i32;

pub fn hci_get(facade_id: FacadeIdentifier) -> Result<ProtoBluetoothChip, String> {
    println!("hci_get({})", facade_id);
    Ok(ProtoBluetoothChip::new())
}

pub fn wifi_get(facade_id: FacadeIdentifier) -> Result<ProtoRadioChip, String> {
    println!("wifi_get({})", facade_id);
    Ok(ProtoRadioChip::new())
}

pub fn hci_remove(facade_id: FacadeIdentifier) -> Result<(), String> {
    println!("hci_remove({})", facade_id);
    Ok(())
}

pub fn wifi_remove(facade_id: FacadeIdentifier) -> Result<(), String> {
    println!("wifi_remove({})", facade_id);
    Ok(())
}

pub fn hci_add(_device_id: DeviceIdentifier) -> Result<FacadeIdentifier, String> {
    println!("hci_add()");
    Ok(0)
}

pub fn wifi_add(_device_id: DeviceIdentifier) -> Result<FacadeIdentifier, String> {
    println!("wifi_add()");
    Ok(0)
}

pub fn hci_patch(_facade_id: FacadeIdentifier, _patch: &ProtoBluetoothChip) -> Result<(), String> {
    println!("hci_patch()");
    Ok(())
}

pub fn wifi_patch(_facade_id: FacadeIdentifier, _patch: &ProtoRadioChip) -> Result<(), String> {
    println!("wifi_patch()");
    Ok(())
}

pub fn hci_reset(_facade_id: FacadeIdentifier) -> Result<(), String> {
    println!("bt_reset()");
    Ok(())
}

pub fn wifi_reset(_facade_id: FacadeIdentifier) -> Result<(), String> {
    println!("wifi_reset()");
    Ok(())
}
