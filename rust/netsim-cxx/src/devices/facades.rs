use crate::proto::model::chip::Bluetooth as ProtoBluetoothChip;
use crate::proto::model::chip::Radio as ProtoRadioChip;

use super::device::DeviceIdentifier;

pub type FacadeIdentifier = i32;

pub fn hci_get(facade_id: FacadeIdentifier) -> ProtoBluetoothChip {
    println!("hci_get({})", facade_id);
    ProtoBluetoothChip::new()
}

pub fn wifi_get(facade_id: FacadeIdentifier) -> ProtoRadioChip {
    println!("wifi_get({})", facade_id);
    ProtoRadioChip::new()
}

pub fn hci_remove(facade_id: FacadeIdentifier) {
    println!("hci_remove({})", facade_id);
}

pub fn wifi_remove(facade_id: FacadeIdentifier) {
    println!("wifi_remove({})", facade_id);
}

pub fn hci_add(_device_id: DeviceIdentifier) -> FacadeIdentifier {
    println!("hci_add()");
    0
}

pub fn wifi_add(_device_id: DeviceIdentifier) -> FacadeIdentifier {
    println!("wifi_add()");
    0
}

pub fn hci_patch(_facade_id: FacadeIdentifier, _patch: &ProtoBluetoothChip) {
    println!("hci_patch()");
}

pub fn wifi_patch(_facade_id: FacadeIdentifier, _patch: &ProtoRadioChip) {
    println!("wifi_patch()");
}

pub fn hci_reset(_facade_id: FacadeIdentifier) {
    println!("bt_reset()");
}

pub fn wifi_reset(_facade_id: FacadeIdentifier) {
    println!("wifi_reset()");
}
