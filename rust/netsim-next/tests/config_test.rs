// Copyright 2023-2025 The Android Open Source Project

//! Integration tests for the `IniFile` manager.

use netsim_next::config::IniFile;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Helper function to create a unique temporary directory for a test.
fn setup_test_dir(test_name: &str) -> PathBuf {
    let dir = env::temp_dir().join(format!("netsim_test_{}_{}", test_name, rand::random::<u32>()));
    fs::create_dir_all(&dir).expect("Failed to create temp dir");
    dir
}

#[test]
fn test_ini_write_and_read_shared() {
    let temp_dir = setup_test_dir("write_and_read");
    let mut ini_writer = IniFile::new(temp_dir.as_path(), "test.ini");
    ini_writer.try_lock().expect("Failed to acquire lock");

    let mut config_data = HashMap::new();
    config_data.insert("port".to_string(), "8080".to_string());
    config_data.insert("host".to_string(), "netsim.google.com".to_string());
    ini_writer.write(&config_data).expect("Failed to write INI file");

    let ini_reader = IniFile::new(temp_dir.as_path(), "test.ini");
    let read_data = ini_reader.read_shared().expect("Failed to read shared INI file");

    assert_eq!(read_data.get("port"), Some(&"8080".to_string()));
    assert_eq!(read_data.get("host"), Some(&"netsim.google.com".to_string()));

    fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_read_shared_handles_malformed_file() {
    let temp_dir = setup_test_dir("malformed_file");
    let ini_path = temp_dir.join("malformed.ini");
    let content = "# This is a comment\n\
                   tcp_port=8080\n\
                   \n\
                   malformed_line_without_equals\n\
                   grpc_port = 9090\n\
                   ; another comment";
    fs::write(&ini_path, content).expect("Failed to write malformed INI file");

    let ini_reader = IniFile::new(temp_dir.as_path(), "malformed.ini");
    let read_result = ini_reader.read_shared();

    assert!(read_result.is_err(), "Reading a malformed file should return an error");
    assert_eq!(read_result.unwrap_err().kind(), io::ErrorKind::InvalidData);

    fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_daemon_mutual_exclusion() {
    let temp_dir = setup_test_dir("mutual_exclusion");
    let mut first_daemon = IniFile::new(temp_dir.as_path(), "netsim.ini");
    assert!(first_daemon.try_lock().is_ok(), "First daemon should acquire the lock");

    let mut second_daemon = IniFile::new(temp_dir.as_path(), "netsim.ini");
    assert!(second_daemon.try_lock().is_err(), "Second daemon should fail to acquire the lock");

    fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_cleanup_removes_lock_file() {
    let temp_dir = setup_test_dir("cleanup_removes_lock");
    let lock_path = temp_dir.join("test.ini.lock");

    let mut ini = IniFile::new(temp_dir.as_path(), "test.ini");
    ini.try_lock().expect("Failed to acquire lock");

    if !Path::new(&lock_path).exists() {
        panic!("Lock file should exist after try_lock()");
    }

    drop(ini);

    if Path::new(&lock_path).exists() {
        panic!("Lock file should be removed after drop()");
    }

    fs::remove_dir_all(&temp_dir).unwrap();
}
