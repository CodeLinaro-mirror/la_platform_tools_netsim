//  Copyright 2021 The Android Open Source Project
//
// SPDX-License-Identifier: Apache-2.0

//! protobuf library for netsim
use protobuf::well_known_types::empty;

pub mod common;
pub mod config;
pub mod configuration;
pub mod frontend;
pub mod frontend_grpc;
pub mod hci_packet;
pub mod model;
pub mod packet_streamer;
pub mod packet_streamer_grpc;
pub mod startup;
pub mod stats;
