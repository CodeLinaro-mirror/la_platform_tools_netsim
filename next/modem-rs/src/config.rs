// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Deserializer};

use crate::{
    apdu,
    constants::{SW_INCORRECT_PARAMS, SW_REFERENCED_DATA_NOT_FOUND, SW_WRONG_LENGTH, UiccFileId},
};

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
        let record_len = record_len
            .or_else(|| UiccFileId::try_from(id).ok().and_then(UiccFileId::default_record_len));
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

    /// Recursively normalizes record lengths for all known record-based
    /// Elementary Files.
    pub fn normalize_record_lengths(&mut self) {
        for file in &mut self.files {
            match file {
                SimFile::ElementaryFile(ef) => {
                    if ef.record_len.is_none() {
                        ef.record_len = UiccFileId::try_from(ef.file_id)
                            .ok()
                            .and_then(UiccFileId::default_record_len);
                    }
                }
                SimFile::DedicatedFile(df) => {
                    df.normalize_record_lengths();
                }
            }
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

    pub fn update_record(
        &mut self,
        id: impl Into<u16>,
        record_number: usize,
        record_data: &[u8],
    ) -> Result<(), RecordUpdateError> {
        let id = id.into();
        let ef = self.find_ef_mut(id).ok_or(RecordUpdateError::FileNotFound)?;
        ef.update_record(record_number, record_data)
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

    /// Recursively normalizes record lengths for all known record-based
    /// Elementary Files.
    pub fn normalize_record_lengths(&mut self) {
        self.master_file.normalize_record_lengths();
    }
}

/// Errors that can occur when updating a record in an Elementary File.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RecordUpdateError {
    InvalidRecordNumber,
    NotRecordBased,
    WrongLength { expected: usize, actual: usize },
    RecordNotFound,
    FileNotFound,
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

    /// Returns the configured record length, or the canonical 3GPP default, or
    /// `None` if transparent.
    pub fn record_len(&self) -> Option<usize> {
        self.record_len.or_else(|| {
            UiccFileId::try_from(self.file_id).ok().and_then(UiccFileId::default_record_len)
        })
    }

    /// Returns the number of complete records stored in this file, or 0 if not
    /// record-based.
    pub fn record_count(&self) -> usize {
        match self.record_len() {
            Some(rec_len) if rec_len > 0 => self.data.len() / rec_len,
            _ => 0,
        }
    }

    /// Returns a 1-based record slice, or `None` if `record_number == 0`,
    /// `record_len` is not set, or the record index exceeds the current
    /// file buffer.
    pub fn record(&self, record_number: usize) -> Option<&[u8]> {
        if record_number == 0 {
            return None;
        }
        let rec_len = self.record_len()?;
        if rec_len == 0 {
            return None;
        }
        let start = (record_number - 1) * rec_len;
        let end = start + rec_len;
        if end <= self.data.len() { Some(&self.data[start..end]) } else { None }
    }

    /// Returns a mutable 1-based record slice, or `None` if `record_number ==
    /// 0`, `record_len` is not set, or the record index exceeds the current
    /// file buffer.
    pub fn record_mut(&mut self, record_number: usize) -> Option<&mut [u8]> {
        if record_number == 0 {
            return None;
        }
        let rec_len = self.record_len()?;
        if rec_len == 0 {
            return None;
        }
        let start = (record_number - 1) * rec_len;
        let end = start + rec_len;
        if end <= self.data.len() { Some(&mut self.data[start..end]) } else { None }
    }

    /// Returns an iterator yielding each record in this linear-fixed file.
    pub fn records(&self) -> impl Iterator<Item = &[u8]> {
        let rec_len = self.record_len().unwrap_or(0);
        let slice: &[u8] = if rec_len > 0 { &self.data } else { &[] };
        let chunk_size = if rec_len > 0 { rec_len } else { 1 };
        slice.chunks_exact(chunk_size)
    }

    /// Updates a 1-based record in-place within a linear-fixed Elementary File.
    ///
    /// Returns `Ok(())` on success, or a `RecordUpdateError` if the record
    /// number is 0, the file is not record-based, the payload length
    /// mismatches the record length, or the record index is out of bounds.
    pub fn update_record(
        &mut self,
        record_number: usize,
        record_data: &[u8],
    ) -> Result<(), RecordUpdateError> {
        if record_number == 0 {
            return Err(RecordUpdateError::InvalidRecordNumber);
        }
        let Some(rec_len) = self.record_len() else {
            return Err(RecordUpdateError::NotRecordBased);
        };
        if rec_len == 0 {
            return Err(RecordUpdateError::NotRecordBased);
        }
        if record_data.len() != rec_len {
            return Err(RecordUpdateError::WrongLength {
                expected: rec_len,
                actual: record_data.len(),
            });
        }
        let Some(rec_slice) = self.record_mut(record_number) else {
            return Err(RecordUpdateError::RecordNotFound);
        };
        rec_slice.copy_from_slice(record_data);
        Ok(())
    }

    /// Updates the content of this Elementary File from an APDU command.
    ///
    /// Returns `Ok(())` on success, or `Err(status_word)` on error.
    pub fn update(
        &mut self,
        command: apdu::Instruction,
        p1: u8,
        p2: u8,
        p3: u8,
        hex_str: &str,
    ) -> Result<(), u16> {
        match command {
            apdu::Instruction::UpdateBinary => {
                let offset = u16::from_be_bytes([p1, p2]) as usize;
                let Ok(new_bytes) = hex::decode(hex_str) else {
                    return Err(SW_INCORRECT_PARAMS);
                };
                if p3 > 0 && new_bytes.len() != p3 as usize {
                    return Err(SW_WRONG_LENGTH);
                }
                if offset + new_bytes.len() > self.data.len() {
                    self.data.resize(offset + new_bytes.len(), 0xFF);
                }
                self.data[offset..offset + new_bytes.len()].copy_from_slice(&new_bytes);
                Ok(())
            }
            apdu::Instruction::UpdateRecord => {
                let Ok(mode) = apdu::RecordMode::try_from(p2) else {
                    return Err(SW_INCORRECT_PARAMS);
                };
                if !mode.is_absolute() {
                    return Err(SW_INCORRECT_PARAMS);
                }
                let Ok(new_bytes) = hex::decode(hex_str) else {
                    return Err(SW_INCORRECT_PARAMS);
                };
                let Some(rec_len) = self.record_len() else {
                    return Err(SW_INCORRECT_PARAMS);
                };
                if rec_len == 0 || p1 == 0 {
                    return Err(SW_INCORRECT_PARAMS);
                }
                if new_bytes.len() != rec_len || (p3 > 0 && p3 as usize != rec_len) {
                    return Err(SW_WRONG_LENGTH);
                }
                let record_num = p1 as usize;
                let Some(rec_slice) = self.record_mut(record_num) else {
                    return Err(SW_REFERENCED_DATA_NOT_FOUND);
                };
                rec_slice.copy_from_slice(&new_bytes);
                Ok(())
            }
            _ => Err(SW_INCORRECT_PARAMS),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elementary_file_update_record_in_place() {
        let mut ef = ElementaryFile { file_id: 0x6F40, record_len: Some(10), data: vec![0xAA; 20] };
        let new_rec1 = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(ef.update_record(1, &new_rec1), Ok(()));
        assert_eq!(&ef.data[..10], &new_rec1);
        assert_eq!(&ef.data[10..], &[0xAA; 10]);

        let new_rec2 = [11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
        assert_eq!(ef.update_record(2, &new_rec2), Ok(()));
        assert_eq!(&ef.data[..10], &new_rec1);
        assert_eq!(&ef.data[10..], &new_rec2);
    }

    #[test]
    fn test_elementary_file_update_record_errors() {
        let mut ef = ElementaryFile { file_id: 0x6F40, record_len: Some(10), data: vec![0xAA; 20] };
        // Invalid record number 0
        assert_eq!(ef.update_record(0, &[0; 10]), Err(RecordUpdateError::InvalidRecordNumber));
        // Wrong length
        assert_eq!(
            ef.update_record(1, &[0; 5]),
            Err(RecordUpdateError::WrongLength { expected: 10, actual: 5 })
        );
        // Record out of bounds
        assert_eq!(ef.update_record(3, &[0; 10]), Err(RecordUpdateError::RecordNotFound));

        // Transparent file (not record-based)
        let mut transparent =
            ElementaryFile { file_id: 0x2FE2, record_len: None, data: vec![0xAA; 10] };
        assert_eq!(transparent.update_record(1, &[0; 10]), Err(RecordUpdateError::NotRecordBased));
    }

    #[test]
    fn test_dedicated_file_update_record() {
        let mut df = DedicatedFile {
            file_id: 0x7F10,
            files: vec![SimFile::ElementaryFile(ElementaryFile {
                file_id: 0x6F40,
                record_len: Some(3),
                data: vec![0xAA; 6],
            })],
        };
        assert_eq!(df.update_record(0x6F40u16, 1, &[0x11, 0x22, 0x33]), Ok(()));
        let ef = df.find_ef(0x6F40u16).unwrap();
        assert_eq!(&ef.data[..3], &[0x11, 0x22, 0x33]);
        assert_eq!(&ef.data[3..], &[0xAA; 3]);

        // File not found in DF
        assert_eq!(
            df.update_record(0x6F38u16, 1, &[0x11, 0x22, 0x33]),
            Err(RecordUpdateError::FileNotFound)
        );
    }

    #[test]
    fn test_elementary_file_records_methods() {
        let mut ef = ElementaryFile {
            file_id: 0x6F40,
            record_len: Some(5),
            data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        };
        assert_eq!(ef.record_len(), Some(5));
        assert_eq!(ef.record_count(), 2);
        assert_eq!(ef.record(1), Some(&[1, 2, 3, 4, 5][..]));
        assert_eq!(ef.record(2), Some(&[6, 7, 8, 9, 10][..]));
        assert_eq!(ef.record(3), None);
        assert_eq!(ef.record(0), None);

        let records: Vec<&[u8]> = ef.records().collect();
        assert_eq!(records, vec![&[1, 2, 3, 4, 5][..], &[6, 7, 8, 9, 10][..]]);

        if let Some(rec) = ef.record_mut(2) {
            rec[0] = 99;
        }
        assert_eq!(ef.record(2), Some(&[99, 7, 8, 9, 10][..]));

        // File without record_len
        let transparent =
            ElementaryFile { file_id: 0x2FE2, record_len: None, data: vec![0x11, 0x22] };
        assert_eq!(transparent.record_len(), None);
        assert_eq!(transparent.record_count(), 0);
        assert_eq!(transparent.record(1), None);
        assert_eq!(transparent.records().count(), 0);

        // Fallback for known record-based file with record_len == None
        let msisdn_none =
            ElementaryFile { file_id: 0x6F40, record_len: None, data: vec![0xFF; 56] };
        assert_eq!(msisdn_none.record_len(), Some(28));
        assert_eq!(msisdn_none.record_count(), 2);
    }

    #[test]
    fn test_normalize_record_lengths() {
        let mut fs = FileSystem {
            master_file: DedicatedFile {
                file_id: 0x3F00,
                files: vec![
                    SimFile::ElementaryFile(ElementaryFile {
                        file_id: 0x2FE2,
                        record_len: None,
                        data: vec![0x11; 10],
                    }),
                    SimFile::DedicatedFile(DedicatedFile {
                        file_id: 0x7F10,
                        files: vec![
                            SimFile::ElementaryFile(ElementaryFile {
                                file_id: 0x6F40,
                                record_len: None,
                                data: Vec::new(),
                            }),
                            SimFile::ElementaryFile(ElementaryFile {
                                file_id: 0x6FC7,
                                record_len: None,
                                data: vec![0xAA; 76],
                            }),
                        ],
                    }),
                ],
            },
        };

        fs.normalize_record_lengths();

        // 0x2FE2 (ICCID) remains None
        let iccid = fs.find_ef(0x2FE2u16).unwrap();
        assert_eq!(iccid.record_len, None);

        // 0x6F40 (MSISDN) populated with Some(28) and data preserved
        let msisdn = fs.find_ef(0x6F40u16).unwrap();
        assert_eq!(msisdn.record_len, Some(28));
        assert_eq!(msisdn.data, Vec::<u8>::new());

        // 0x6FC7 (MBDN) populated with Some(38) and data preserved
        let mbdn = fs.find_ef(0x6FC7u16).unwrap();
        assert_eq!(mbdn.record_len, Some(38));
        assert_eq!(mbdn.data, vec![0xAA; 76]);
    }

    #[test]
    fn test_elementary_file_update_rejects_update_record_on_transparent() {
        let mut transparent =
            ElementaryFile { file_id: 0x2FE2, record_len: None, data: vec![0x00; 10] };
        let res = transparent.update(
            crate::apdu::Instruction::UpdateRecord,
            1,
            4,
            10,
            "AABBCCDDEEFF00112233",
        );
        assert_eq!(res, Err(SW_INCORRECT_PARAMS));
        assert_eq!(transparent.data, vec![0x00; 10]);
    }
}
