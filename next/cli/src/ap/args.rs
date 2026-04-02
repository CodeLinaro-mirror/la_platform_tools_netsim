// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

#[derive(Debug, Subcommand, PartialEq)]
pub enum ApCommand {
    /// List all current Access Points
    List(ApList),
    /// Create a new Access Point
    Create(ApCreate),
    /// Modify an existing Access Point
    Patch(ApPatch),
    /// Remove an Access Point
    #[command(alias("delete"))]
    Remove(ApRemove),
    /// Disconnect a client from an Access Point
    Disconnect(ApDisconnect),
}

#[derive(Debug, Args, PartialEq, Default)]
pub struct ApList {
    /// Continuously print AP information every second
    #[arg(short, long)]
    pub continuous: bool,
}

#[derive(Debug, Args, PartialEq)]
pub struct ApCreate {
    /// Name (SSID) of the Access Point
    #[arg(long)]
    pub ssid: Option<String>,
    /// Wi-Fi Channel
    #[arg(long)]
    pub channel: Option<u32>,
    /// BSSID (MAC Address)
    #[arg(long)]
    pub bssid: Option<String>,
    /// WPA2 Personal Password
    #[arg(long)]
    pub password: Option<String>,
}

#[derive(Debug, Args, PartialEq)]
pub struct ApPatch {
    /// ID of the Access Point
    pub id: u32,
    /// New Name (SSID)
    #[arg(long)]
    pub ssid: Option<String>,
    /// New Wi-Fi Channel
    #[arg(long)]
    pub channel: Option<u32>,
}

#[derive(Debug, Args, PartialEq)]
pub struct ApRemove {
    /// ID of the Access Point to remove
    pub id: u32,
}

#[derive(Debug, Args, PartialEq)]
pub struct ApDisconnect {
    /// ID of the Access Point
    pub id: u32,
    /// MAC Address of the client to disconnect
    pub mac_address: String,
}
