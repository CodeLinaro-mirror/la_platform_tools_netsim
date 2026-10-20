// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use clap::Subcommand;

#[derive(Debug, Subcommand, PartialEq)]
pub enum NfcCommand {
    /// List all virtual NFC devices and their radio status
    #[command(visible_alias("status"))]
    List {
        /// Optional chip ID to inspect
        #[arg(short, long)]
        chip_id: Option<u32>,
        /// Output in JSON format
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Execute NFC polling sequence against simulation scene
    Poll {
        /// Target chip ID initiating poll
        #[arg(default_value_t = 0)]
        chip_id: u32,
    },
    /// Transmit hex-encoded APDU packet to target chip
    Apdu {
        /// Target chip ID
        chip_id: u32,
        /// Hex-encoded APDU command payload
        payload: String,
    },
    /// Toggle RF antenna power state for a chip
    #[command(visible_alias("set-power"))]
    Radio {
        /// Target chip ID
        #[arg(short, long, default_value_t = 0)]
        chip_id: u32,
        /// Power state: on/true/1 or off/false/0
        state: String,
    },
}
