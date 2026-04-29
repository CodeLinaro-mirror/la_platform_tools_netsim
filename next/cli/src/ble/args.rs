// Copyright 2026 The Android Open Source Project

use clap::{Args, Subcommand};

#[derive(Debug, Subcommand, PartialEq)]
pub enum BleCommand {
    /// Start a Bluetooth Low Energy scan
    Scan(BleScan),
    /// Start a baseband packet sniffer
    #[command(aliases = ["sniffer", "snif"])]
    Sniff(BleSniff),
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
    /// Send Active Scan Requests (SCAN_REQ) to trigger Scan Responses from
    /// beacons
    #[arg(long)]
    pub active: bool,
    /// Output raw binary PCAP stream to stdout for piping to tshark/Wireshark
    #[arg(long)]
    pub pcap: bool,
}

#[derive(Debug, Args, PartialEq)]
pub struct BleSniff {
    /// X position of the sniffer
    #[arg(long, default_value = "0.0")]
    pub x: f32,
    /// Y position of the sniffer
    #[arg(long, default_value = "0.0")]
    pub y: f32,
    /// Z position of the sniffer
    #[arg(long, default_value = "0.0")]
    pub z: f32,
    /// Output to a PCAP file
    #[arg(long, num_args(0..=1), default_missing_value = "-")]
    pub pcap: Option<String>,
    /// Link Type of the PCAP capture
    #[arg(long, value_enum, ignore_case = true, default_value_t = LinkTypeOption::Le)]
    pub link_type: LinkTypeOption,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum LinkTypeOption {
    /// Classic Bluetooth Baseband (LINKTYPE_BLUETOOTH_BREDR_BB)
    Bredr,
    /// Low Energy Link Layer (LINKTYPE_BLUETOOTH_LE_LL)
    Le,
}
