//  Copyright 2021 The Android Open Source Project
//
// SPDX-License-Identifier: Apache-2.0

//! protobuf library for netsim
#![allow(renamed_and_removed_lints)]
pub use protobuf;
pub use protobuf::well_known_types::empty;

pub mod access_point;
pub mod access_point_grpc;
pub mod ble_service;
pub mod ble_service_grpc;
pub mod casimir_control;
pub mod casimir_control_grpc;
pub mod cell;
pub mod cell_grpc;
pub mod common;
pub mod config;
pub mod configuration;
pub mod frontend;
pub mod frontend_grpc;
pub mod hci_packet;
pub mod model;
pub mod nfc_service;
pub mod nfc_service_grpc;
pub mod packet_streamer;
pub mod packet_streamer_grpc;
pub mod startup;
pub mod stats;

pub use casimir_control as casimircontrolserver;

/// Google protobuf well-known types wrapper for gRPC compatibility
#[allow(missing_docs)]
pub mod google {
    /// Protobuf well-known types
    #[allow(missing_docs)]
    pub mod protobuf {
        pub use crate::protobuf::well_known_types::empty::Empty;
        pub use crate::protobuf::well_known_types::timestamp::Timestamp;
    }
}
