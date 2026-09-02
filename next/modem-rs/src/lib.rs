// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

mod apdu;
mod call_service;
pub mod config;
pub mod constants;
mod cuttlefish;
mod data_service;
mod metrics;
mod misc_service;
mod modem;
pub mod modem_network;
mod modem_network_simulator;
mod network_service;
mod parser;
mod pdu;
pub mod profiles;
mod sim_service;
mod sms_service;
mod stk_service;
mod sup_service;
pub mod time;

pub mod xml_profile;

pub mod test_utils;
mod types; // Test utils might need to be public for integration tests

// The Public API
// Configuration types needed for setup
pub use config::{
    DedicatedFile, ElementaryFile, FileSystem, PinProfile, PinState, ProfileMetadata, SimFile,
    SimIo, SimProfile,
};
pub use metrics::MetricsSnapshot;
pub use modem::ModemEvent;
pub use modem_network_simulator::{ModemNetworkSimulator, NetworkEvent, ScheduledEvent};
pub use netsim_model::RegistrationStatus;
pub use pdu::{EncodeError, ParseError, SubmitPdu};
pub use profiles::{
    SIM_TYPE_CTS, SIM_TYPE_DEFAULT, SIM_TYPE_TEL_ALASKA, XmlProfileError, get_builtin_profile,
    parse_xml_profile,
};
pub use types::{AT_ERROR, AT_OK, HostEvent, ModemError, ModemId, ModemSink, Plmn, PlmnError};
