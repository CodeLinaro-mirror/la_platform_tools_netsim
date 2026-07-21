//  Copyright 2023 The Android Open Source Project
//
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{HashMap, hash_map::Entry},
    fs::read_to_string,
    path::PathBuf,
};

use tracing::error;

use super::os_utils::get_discovery_directory;

#[derive(Default)]
pub struct IniParserOptions {
    pub strict: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum IniParseErrorKind {
    #[error("Empty key")]
    EmptyKey,
    #[error("Missing `=`")]
    MissingDelimiter,
    #[error("Duplicate key `{key}` with conflicting values: `{existing_value}` vs `{new_value}`")]
    DuplicateKey { key: String, existing_value: String, new_value: String },
}

#[derive(Debug, thiserror::Error)]
#[error("{line_number}:{column_number} {kind} ({line_content})")]
pub struct IniParseError {
    pub line_number: usize,
    pub column_number: usize,
    pub kind: IniParseErrorKind,
    pub line_content: String,
}

pub fn parse_ini(
    content: &str,
    options: &IniParserOptions,
) -> Result<HashMap<String, String>, IniParseError> {
    let mut map: HashMap<String, String> = HashMap::new();
    for (line_idx, line) in content.lines().enumerate() {
        let line_number = line_idx + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            if options.strict && key.is_empty() {
                let column_number = line.find('=').unwrap_or(0) + 1;
                return Err(IniParseError {
                    line_number,
                    column_number,
                    kind: IniParseErrorKind::EmptyKey,
                    line_content: line.to_string(),
                });
            }

            let entry = map.entry(key.to_string());
            if options.strict
                && let Entry::Occupied(ref occupied) = entry
            {
                let existing_value = occupied.get();
                if existing_value != value {
                    let column_number = line.find(key).unwrap_or(0) + 1;
                    return Err(IniParseError {
                        line_number,
                        column_number,
                        kind: IniParseErrorKind::DuplicateKey {
                            key: key.to_string(),
                            existing_value: existing_value.to_string(),
                            new_value: value.to_string(),
                        },
                        line_content: line.to_string(),
                    });
                }
            }
            entry.insert_entry(value.to_string());
        } else if options.strict {
            let column_number = line.find(trimmed).unwrap_or(0) + 1;
            return Err(IniParseError {
                line_number,
                column_number,
                kind: IniParseErrorKind::MissingDelimiter,
                line_content: line.to_string(),
            });
        }
    }
    Ok(map)
}

/// Get the filename of netsim.ini for a given instance number.
pub fn get_ini_filename(instance_num: u16) -> String {
    if instance_num == 1 { "netsim.ini".to_string() } else { format!("netsim_{instance_num}.ini") }
}

/// Get the filepath of netsim.ini under discovery directory
pub fn get_ini_filepath(instance_num: u16) -> PathBuf {
    let mut discovery_dir = get_discovery_directory();
    discovery_dir.push(get_ini_filename(instance_num));
    discovery_dir
}

/// Get the grpc server address for netsim
pub fn get_server_address(instance_num: u16) -> Option<String> {
    get_address_by_key(instance_num, "grpc.port")
}

/// Get the TCP server address for netsim packet stream
pub fn get_tcp_server_address(instance_num: u16) -> Option<String> {
    get_address_by_key(instance_num, "tcp.port")
}

fn get_address_by_key(instance_num: u16, key: &str) -> Option<String> {
    let filepath = get_ini_filepath(instance_num);
    if !filepath.exists() {
        error!("Unable to find netsim ini file: {filepath:?}");
        return None;
    }
    if !filepath.is_file() {
        error!("Not a file: {filepath:?}");
        return None;
    }
    let ini_file_contents = read_to_string(filepath)
        .inspect_err(|err| {
            error!("Error reading ini file: {err}");
        })
        .ok()?;
    let ini_map = parse_ini(&ini_file_contents, &IniParserOptions { strict: false })
        .inspect_err(|err| {
            error!("Error parsing ini file: {err}");
        })
        .ok()?;
    // Return the address constructed from ini_file. Format: localhost:{port}
    // If it starts with a port like "8888", we prepend localhost: to it.
    // If it's already a full address like ":8888" or "127.0.0.1:8888" we return it
    // as is.
    ini_map.get(key).map(|s| if s.contains(':') { s.to_string() } else { format!("localhost:{s}") })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{get_ini_filepath, parse_ini};
    use crate::tests::ENV_MUTEX;

    #[test]
    fn test_read() {
        for test_case in ["port=123", "port= 123", "port =123", " port = 123 "] {
            let ini_map = parse_ini(test_case, &Default::default()).unwrap();
            assert_eq!(ini_map.get("port").unwrap(), "123");
        }
    }

    #[test]
    fn test_read_no_newline() {
        let ini_map = parse_ini("port=123", &Default::default()).unwrap();
        assert_eq!(ini_map.get("port").unwrap(), "123");
    }

    #[test]
    fn test_read_multiple_lines() {
        let ini_map = parse_ini("port=123\nport2=456\n", &Default::default()).unwrap();
        assert_eq!(ini_map.get("port").unwrap(), "123");
        assert_eq!(ini_map.get("port2").unwrap(), "456");
    }

    #[test]
    fn test_get_ini_filepath() {
        let _locked = ENV_MUTEX.lock();

        // Test with TMPDIR variable
        // SAFETY: Serialized via ENV_MUTEX.
        unsafe {
            std::env::set_var("TMPDIR", "/tmpdir");
        }

        // Test get_netsim_ini_filepath
        assert_eq!(get_ini_filepath(1), PathBuf::from("/tmpdir/netsim.ini"));
        assert_eq!(get_ini_filepath(2), PathBuf::from("/tmpdir/netsim_2.ini"));
    }
    #[test]
    fn test_parse_ini_strict() {
        use super::{IniParseErrorKind, IniParserOptions, parse_ini};

        // Test empty key
        let content = "=value";
        let result = parse_ini(content, &IniParserOptions { strict: true });
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.line_number, 1);
        assert_eq!(err.column_number, 1);
        assert!(matches!(err.kind, IniParseErrorKind::EmptyKey));
        assert_eq!(err.line_content, "=value");

        // Test missing delimiter
        let content = "key_without_equals";
        let result = parse_ini(content, &IniParserOptions { strict: true });
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.line_number, 1);
        assert_eq!(err.column_number, 1);
        assert!(matches!(err.kind, IniParseErrorKind::MissingDelimiter));
        assert_eq!(err.line_content, "key_without_equals");

        // Test duplicate key
        let content = "key=value1\nkey=value2";
        let result = parse_ini(content, &IniParserOptions { strict: true });
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.line_number, 2);
        assert_eq!(err.column_number, 1);
        if let IniParseErrorKind::DuplicateKey { key, existing_value, new_value } = err.kind {
            assert_eq!(key, "key");
            assert_eq!(existing_value, "value1");
            assert_eq!(new_value, "value2");
        } else {
            panic!("Expected DuplicateKey error");
        }
        assert_eq!(err.line_content, "key=value2");

        // Test duplicate key with same value passes
        let content = "key=value1\nkey=value1";
        let result = parse_ini(content, &IniParserOptions { strict: true });
        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(map.get("key").unwrap(), "value1");
        assert_eq!(map.len(), 1);

        // Test non-strict mode ignores these
        let content = "=value\nkey_without_equals\nkey=value1\nkey=value2";
        let result = parse_ini(content, &IniParserOptions { strict: false });
        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(map.get("").unwrap(), "value");
        assert_eq!(map.get("key").unwrap(), "value2");
        assert_eq!(map.len(), 2);
    }
}
