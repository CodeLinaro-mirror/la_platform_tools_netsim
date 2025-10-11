// Copyright (C) 2023-2025 The Android Open Source Project

//! Translates between the public protobuf API models and the internal
//! simulation models for BLE beacons.
//!
//! This module is responsible for converting the `BleBeacon` protobuf message,
//! which is part of the public API, into an `Advertiser` object that is used
//! by the `ble-advertisers` simulation backend. This separation of concerns
//! ensures that the public API is decoupled from the internal implementation
//! details of the simulation.
//!
//! The conversion process involves translating advertising settings, data
//! packets, and other beacon properties into their corresponding internal
// representations.
