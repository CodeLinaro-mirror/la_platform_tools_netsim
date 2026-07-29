// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/config.rs

use serde::{Deserialize, Deserializer};

fn deserialize_hex_u16<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    u16::from_str_radix(&s, 16).map_err(serde::de::Error::custom)
}

/// Represents the SIM profile configuration.
#[derive(Debug, Deserialize, Default)]
pub struct SimProfile {
    pub iccid: String,
    pub imsi: String,
    pub msisdn: String,
    pub pin_profile: PinProfile,
    pub facility_locks: FacilityLocks,
    pub stk: Stk,
    pub sim_io: SimIo,
    pub enable_unsolicited_urcs: Option<bool>,
}

/// Represents the PIN profile configuration.
#[derive(Debug, Deserialize, Default)]
pub struct PinProfile {
    pub state: String,
    pub pin1: String,
    pub puk1: String,
    pub pin2: String,
    pub puk2: String,
}

/// Represents the facility locks configuration.
#[derive(Debug, Deserialize, Default)]
pub struct FacilityLocks {
    pub sim_lock: String,
    pub fixed_dialing: String,
}

/// Represents the SIM Application Toolkit (STK) configuration.
#[derive(Debug, Deserialize, Default)]
pub struct Stk {
    pub setup_menu: StkMenuItem,
}

/// Represents a single STK menu item.
#[derive(Debug, Deserialize, Default)]
pub struct StkMenuItem {
    pub text: String,
    #[serde(default)]
    pub items: Vec<StkMenuItem>,
}

/// Represents the SIM I/O configuration.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct SimIo {
    #[serde(flatten)]
    pub file_system: FileSystem,
}

/// Represents the entire SIM file system, starting from the Master File.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct FileSystem {
    #[serde(rename = "master_file")]
    pub master_file: DedicatedFile,
}

/// Represents a file on the SIM, which can be a Dedicated File (directory)
/// or an Elementary File (data file).
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum SimFile {
    Df(DedicatedFile),
    Ef(ElementaryFile),
}

impl SimFile {
    pub fn as_df(&self) -> Option<&DedicatedFile> {
        match self {
            SimFile::Df(df) => Some(df),
            _ => None,
        }
    }

    pub fn as_ef(&self) -> Option<&ElementaryFile> {
        match self {
            SimFile::Ef(ef) => Some(ef),
            _ => None,
        }
    }
}

/// Represents a Dedicated File (DF), which is a directory in the SIM file
/// system.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct DedicatedFile {
    #[serde(deserialize_with = "deserialize_hex_u16")]
    pub file_id: u16,
    #[serde(default)]
    pub files: Vec<SimFile>,
}

/// Represents an Elementary File (EF), which contains the actual data.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct ElementaryFile {
    #[serde(deserialize_with = "deserialize_hex_u16")]
    pub file_id: u16,
    #[serde(default)]
    pub size: usize,
    #[serde(default)]
    pub record_len: Option<usize>,
    #[serde(default)]
    pub data: String,
}
