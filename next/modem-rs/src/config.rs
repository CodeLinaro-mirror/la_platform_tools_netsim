// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Deserializer};

fn deserialize_hex_u16<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    u16::from_str_radix(&s, 16).map_err(serde::de::Error::custom)
}

/// Represents the SIM profile configuration.
#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct SimProfile {
    pub iccid: String,
    pub imsi: String,
    pub msisdn: String,
    pub pin_profile: PinProfile,
    pub facility_locks: FacilityLocks,
    pub stk: Stk,
    pub sim_io: SimIo,
    pub enable_unsolicited_urcs: Option<bool>,
    pub eid: Option<String>,
    pub atr: Option<String>,
    #[serde(default)]
    pub adfs: Vec<ApplicationDedicatedFile>,
}

impl SimProfile {
    /// Returns the home PLMN (MCC + MNC) derived from the SIM's IMSI.
    ///
    /// Extracts the 6-digit PLMN prefix (assuming a 3-digit MNC, as used by all
    /// standard emulator test profiles).
    /// Note: Supporting 2-digit MNCs for arbitrary 15-digit IMSIs requires
    /// reading the MNC length from EF_AD (0x6FAD).
    /// Falls back to `DEFAULT_PLMN` if `imsi` is missing or unpopulated.
    pub fn home_plmn(&self) -> &str {
        if self.imsi.len() >= 6 { &self.imsi[..6] } else { crate::constants::DEFAULT_PLMN }
    }
}

/// Represents the PIN profile configuration.
#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct PinProfile {
    pub state: String,
    pub pin1: String,
    pub puk1: String,
    pub pin2: String,
    pub puk2: String,
    pub pin1_retries: Option<u32>,
    pub puk1_retries: Option<u32>,
    pub pin2_retries: Option<u32>,
    pub puk2_retries: Option<u32>,
}

/// Represents the facility locks configuration.
#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct FacilityLocks {
    pub sim_lock: String,
    pub fixed_dialing: String,
}

/// Represents the SIM Application Toolkit (STK) configuration.
#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct Stk {
    pub setup_menu: StkMenuItem,
}

/// Represents a single STK menu item.
#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct StkMenuItem {
    #[serde(default)]
    pub id: u8,
    #[serde(default)]
    pub menu_id: u8,
    pub text: String,
    #[serde(default)]
    pub items: Vec<StkMenuItem>,
}

/// Represents the SIM I/O configuration.
#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct SimIo {
    #[serde(flatten)]
    pub file_system: FileSystem,
}

/// Represents the entire SIM file system, starting from the Master File.
#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct FileSystem {
    #[serde(rename = "master_file")]
    pub master_file: DedicatedFile,
}

/// Represents a file on the SIM, which can be a Dedicated File (directory)
/// or an Elementary File (data file).
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum SimFile {
    DedicatedFile(DedicatedFile),
    ElementaryFile(ElementaryFile),
}

impl SimFile {
    pub fn as_df(&self) -> Option<&DedicatedFile> {
        match self {
            SimFile::DedicatedFile(df) => Some(df),
            _ => None,
        }
    }

    pub fn as_ef(&self) -> Option<&ElementaryFile> {
        match self {
            SimFile::ElementaryFile(ef) => Some(ef),
            _ => None,
        }
    }
}

/// Represents a Dedicated File (DF), which is a directory in the SIM file
/// system.
#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct DedicatedFile {
    #[serde(deserialize_with = "deserialize_hex_u16")]
    pub file_id: u16,
    #[serde(default)]
    pub files: Vec<SimFile>,
}

/// Policy for overwriting existing file content when provisioning files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverwritePolicy {
    Always,
    IfUninitialized,
    Never,
}

impl DedicatedFile {
    pub fn ensure_ef_present(
        &mut self,
        id: impl Into<u16>,
        record_len: Option<usize>,
        data: Vec<u8>,
        policy: OverwritePolicy,
    ) {
        let id = id.into();
        if let Some(ef) = self.find_ef_mut(id) {
            let should_overwrite = match policy {
                OverwritePolicy::Always => true,
                OverwritePolicy::IfUninitialized => ef.data.iter().all(|&b| b == 0xFF),
                OverwritePolicy::Never => false,
            };
            if should_overwrite {
                ef.data = data;
                ef.record_len = record_len;
            }
        } else {
            self.files.push(SimFile::ElementaryFile(ElementaryFile {
                file_id: id,
                record_len,
                data,
            }));
        }
    }

    pub fn find_df(&self, id: impl Into<u16>) -> Option<&DedicatedFile> {
        let id = id.into();
        if self.file_id == id {
            return Some(self);
        }
        for file in &self.files {
            if let SimFile::DedicatedFile(sub_df) = file
                && let Some(found) = sub_df.find_df(id)
            {
                return Some(found);
            }
        }
        None
    }

    pub fn find_df_mut(&mut self, id: impl Into<u16>) -> Option<&mut DedicatedFile> {
        let id = id.into();
        if self.file_id == id {
            return Some(self);
        }
        for file in &mut self.files {
            if let SimFile::DedicatedFile(sub_df) = file
                && let Some(found) = sub_df.find_df_mut(id)
            {
                return Some(found);
            }
        }
        None
    }

    pub fn find_ef(&self, id: impl Into<u16>) -> Option<&ElementaryFile> {
        let id = id.into();
        for file in &self.files {
            if let SimFile::ElementaryFile(ef) = file
                && ef.file_id == id
            {
                return Some(ef);
            }
            if let SimFile::DedicatedFile(df) = file
                && let Some(found) = df.find_ef(id)
            {
                return Some(found);
            }
        }
        None
    }

    pub fn find_ef_mut(&mut self, id: impl Into<u16>) -> Option<&mut ElementaryFile> {
        let id = id.into();
        for file in &mut self.files {
            match file {
                SimFile::ElementaryFile(ef) => {
                    if ef.file_id == id {
                        return Some(ef);
                    }
                }
                SimFile::DedicatedFile(df) => {
                    if let Some(ef) = df.find_ef_mut(id) {
                        return Some(ef);
                    }
                }
            }
        }
        None
    }
}

impl FileSystem {
    pub fn find_df(&self, id: impl Into<u16>) -> Option<&DedicatedFile> {
        self.master_file.find_df(id)
    }

    pub fn find_df_mut(&mut self, id: impl Into<u16>) -> Option<&mut DedicatedFile> {
        self.master_file.find_df_mut(id)
    }

    pub fn find_ef(&self, id: impl Into<u16>) -> Option<&ElementaryFile> {
        self.master_file.find_ef(id)
    }

    pub fn find_ef_mut(&mut self, id: impl Into<u16>) -> Option<&mut ElementaryFile> {
        self.master_file.find_ef_mut(id)
    }

    pub fn ensure_ef_present(
        &mut self,
        id: impl Into<u16>,
        record_len: Option<usize>,
        data: Vec<u8>,
        policy: OverwritePolicy,
    ) {
        self.master_file.ensure_ef_present(id, record_len, data, policy);
    }
}

/// Represents an Elementary File (EF), which contains the actual data.
#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct ElementaryFile {
    #[serde(deserialize_with = "deserialize_hex_u16")]
    pub file_id: u16,

    #[serde(default)]
    pub record_len: Option<usize>,
    #[serde(default)]
    pub data: Vec<u8>,
}

impl ElementaryFile {
    /// Returns the size of the file in bytes.
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct ApduMapping {
    pub cmd: crate::apdu::ParsedApdu<'static>,
    pub response: String,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
pub struct ApplicationFileOverride {
    #[serde(deserialize_with = "deserialize_hex_u16")]
    pub id: u16,
    pub cgla: Vec<ApduMapping>,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
pub struct ApplicationDedicatedFile {
    pub aid: String,
    #[serde(default)]
    pub cgla: Vec<ApduMapping>,
    #[serde(default)]
    pub csim: Vec<ApduMapping>,
    #[serde(default)]
    pub files: Vec<ApplicationFileOverride>,
}
