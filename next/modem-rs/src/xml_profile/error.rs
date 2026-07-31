// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, PartialEq, Eq)]
pub enum XmlProfileError {
    Deserialization(String),
    MissingMasterFile,
    InvalidSimIoResponse { file_id: u16, response: String },
    InvalidValue { file_id: u16, field: String, value: String, expected: String },
}

impl std::fmt::Display for XmlProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Deserialization(e) => write!(f, "XML deserialization error: {e}"),
            Self::MissingMasterFile => write!(f, "Missing Master File (MF) in SIM profile XML"),
            Self::InvalidSimIoResponse { file_id, response } => {
                write!(f, "Invalid SIMIO B0/B2 response format for file {file_id:04X}: {response}")
            }
            Self::InvalidValue { file_id, field, value, expected } => {
                write!(
                    f,
                    "Invalid value '{value}' in field '{field}' for file '{file_id:04X}'. Expected: {expected}"
                )
            }
        }
    }
}

impl std::error::Error for XmlProfileError {}

impl From<serde_xml_rs::Error> for XmlProfileError {
    fn from(e: serde_xml_rs::Error) -> Self {
        Self::Deserialization(e.to_string())
    }
}

impl From<XmlProfileError> for crate::types::ModemError {
    fn from(err: XmlProfileError) -> Self {
        crate::types::ModemError::InvalidConfig(err.to_string())
    }
}
