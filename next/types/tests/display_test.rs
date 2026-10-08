// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_types::{ChipInfo, ChipKind};

#[test]
fn test_chip_kind_display() {
    assert_eq!(format!("{}", ChipKind::BLUETOOTH), "BLUETOOTH");
    assert_eq!(format!("{}", ChipKind::WIFI), "WIFI");
    assert_eq!(format!("{}", ChipKind::UWB), "UWB");
    assert_eq!(format!("{}", ChipKind::NFC), "NFC");
    assert_eq!(format!("{}", ChipKind::CELLULAR), "CELLULAR");
    assert_eq!(format!("{}", ChipKind::CELLULAR_DATA), "CELLULAR_DATA");
    assert_eq!(format!("{}", ChipKind::ETHERNET), "ETHERNET");
}

#[test]
fn test_chip_info_display() {
    let chip_info = ChipInfo::new("Pixel 9 Pro", ChipKind::BLUETOOTH);
    assert_eq!(format!("{chip_info}"), "BLUETOOTH on device Pixel 9 Pro");

    let no_chip = ChipInfo { name: "test-device".to_string(), chip: None, device_info: None };
    assert_eq!(format!("{no_chip}"), "UNSPECIFIED on device Unknown");
}
