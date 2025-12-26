// src/config.rs

use serde::Deserialize;

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

/// Represents a Dedicated File (DF), which is a directory in the SIM file system.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct DedicatedFile {
    pub file_id: String,
    #[serde(default)]
    pub files: Vec<SimFile>,
}

/// Represents an Elementary File (EF), which contains the actual data.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct ElementaryFile {
    pub file_id: String,
    #[serde(default)]
    pub size: usize,
    #[serde(default)]
    pub data: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_sim_profile() {
        let toml_str = r#"
            iccid = "123"
            imsi = "456"
            msisdn = "789"
            [pin_profile]
            state = "Ready"
            pin1 = "1234"
            puk1 = "12345678"
            pin2 = "1234"
            puk2 = "12345678"
            [facility_locks]
            sim_lock = "Disabled"
            fixed_dialing = "Disabled"
            [stk]
            [stk.setup_menu]
            text = "Menu"
            items = []
            [sim_io.master_file]
            file_id = "3F00"
            files = []
        "#;
        let profile: SimProfile = toml::from_str(toml_str).unwrap();
        assert_eq!(profile.iccid, "123");
    }

    #[test]
    fn test_deserialize_file_system() {
        let toml_str = r#"
            [master_file]
            file_id = "3F00"

            [[master_file.files]]
            type = "ef"
            file_id = "2F00"
            size = 12
            data = "010203040506070809101112"

            [[master_file.files]]
            type = "df"
            file_id = "7F20"

            [[master_file.files.files]]
            type = "ef"
            file_id = "6F07"
            size = 10
            data = "AABBCCDDEEFF99887766"
        "#;

        let fs: FileSystem = toml::from_str(toml_str).unwrap();
        assert_eq!(fs.master_file.file_id, "3F00");
        assert_eq!(fs.master_file.files.len(), 2);

        let ef = fs.master_file.files[0].as_ef().unwrap();
        assert_eq!(ef.file_id, "2F00");
        assert_eq!(ef.data, "010203040506070809101112");

        let df = fs.master_file.files[1].as_df().unwrap();
        assert_eq!(df.file_id, "7F20");
        assert_eq!(df.files.len(), 1);

        let nested_ef = df.files[0].as_ef().unwrap();
        assert_eq!(nested_ef.file_id, "6F07");
    }
}
