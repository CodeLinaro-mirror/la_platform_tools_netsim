// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

#[derive(Debug, Subcommand, PartialEq)]
pub enum GsmCommand {
    /// Show registration status and signal strength (gsm status)
    Status(GsmStatus),
    /// List all simulated cellular devices (netsim specific)
    List,
    /// Trigger an incoming call (gsm call <number>)
    Call(GsmCall),
    /// Simulate remote party answering an outbound call (gsm accept <number>)
    Accept(GsmAccept),
    /// Reject incoming call or terminate active call (gsm cancel <number>)
    Cancel(GsmCancel),
    /// Place a call on hold or take it off hold (gsm hold <number>)
    Hold(GsmHold),
    /// Set signal strength (gsm signal <rssi> [ber])
    Signal(GsmSignal),
    /// Set voice registration status (gsm voice <state>)
    Voice(GsmVoice),
    /// Set data registration status (gsm data <state>)
    Data(GsmData),
}

#[derive(Debug, Args, PartialEq)]
pub struct GsmStatus {
    /// ID of the cellular device
    #[arg(long)]
    pub id: Option<u32>,
}

#[derive(Debug, Args, PartialEq)]
pub struct GsmCall {
    /// Phone number of the incoming call (digits, optional leading '+', max 15
    /// digits)
    pub number: String,
    /// ID of the cellular device
    #[arg(long)]
    pub id: Option<u32>,
}

#[derive(Debug, Args, PartialEq)]
pub struct GsmAccept {
    /// Phone number of the call to accept (digits, optional leading '+', max 15
    /// digits)
    pub number: String,
    /// ID of the cellular device
    #[arg(long)]
    pub id: Option<u32>,
}

#[derive(Debug, Args, PartialEq)]
pub struct GsmCancel {
    /// Phone number of the call to cancel (digits, optional leading '+', max 15
    /// digits)
    pub number: String,
    /// ID of the cellular device
    #[arg(long)]
    pub id: Option<u32>,
}

#[derive(Debug, Args, PartialEq)]
pub struct GsmHold {
    /// Phone number of the call (digits, optional leading '+', max 15 digits)
    pub number: String,
    /// Hold state (on/off). If not specified, toggles hold state.
    #[arg(value_enum, ignore_case = true)]
    pub state: Option<HoldStateOption>,
    /// ID of the cellular device
    #[arg(long)]
    pub id: Option<u32>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum HoldStateOption {
    On,
    Off,
}

#[derive(Debug, Args, PartialEq)]
pub struct GsmSignal {
    /// RSSI value (0-31, or 99 for unknown)
    pub rssi: u32,
    /// BER value (0-7, or 99 for unknown)
    #[arg(default_value = "99")]
    pub ber: u32,
    /// ID of the cellular device
    #[arg(long)]
    pub id: Option<u32>,
}

#[derive(Debug, Args, PartialEq)]
pub struct GsmVoice {
    /// Registration status
    #[arg(value_enum, ignore_case = true)]
    pub status: RegistrationStatusOption,
    /// ID of the cellular device
    #[arg(long)]
    pub id: Option<u32>,
}

#[derive(Debug, Args, PartialEq)]
pub struct GsmData {
    /// Registration status
    #[arg(value_enum, ignore_case = true)]
    pub status: RegistrationStatusOption,
    /// ID of the cellular device
    #[arg(long)]
    pub id: Option<u32>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum RegistrationStatusOption {
    Unregistered,
    Home,
    Searching,
    Denied,
    Unknown,
    Roaming,
}
