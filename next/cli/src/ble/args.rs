// Copyright 2026 The Android Open Source Project

use clap::{Args, Subcommand};

#[derive(Debug, Subcommand, PartialEq)]
pub enum BleCommand {
    /// Start a Bluetooth Low Energy scan
    Scan(BleScan),
}

#[derive(Debug, Args, PartialEq)]
pub struct BleScan {
    /// X position of the scanner
    #[arg(long, default_value = "0.0")]
    pub x: f32,
    /// Y position of the scanner
    #[arg(long, default_value = "0.0")]
    pub y: f32,
    /// Z position of the scanner
    #[arg(long, default_value = "0.0")]
    pub z: f32,
}
