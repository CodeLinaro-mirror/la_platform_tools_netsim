// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Manages the `netsim.ini` file for inter-process discovery and configuration.
//!
//! This module provides a mechanism to ensure only one primary instance of the
//! netsim daemon is running using a file-based lock on the ini file.
//! It also handles reading and writing configuration parameters (like PID and
//! gRPC port) to the `netsim.ini` file, located in a platform-specific runtime
//! directory.
//!
//! The `IniFile::try_acquire` method is the main entry point, determining if
//! the current process becomes the `Writer` (gets the lock, can write the INI
//! file) or a `Reader` (another process holds the lock, can only read the INI
//! file).

use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions, TryLockError},
    io::{self, BufWriter, Write},
    path::PathBuf,
    str::FromStr,
};

use common::util::os_utils::get_discovery_directory;
use tracing::{debug, warn};

// --- INI File Management ---

/// Discovery configuration (must remain readable by all instances).
const INI_FILENAME: &str = "netsim.ini";

/// File used for primary instance synchronization.
/// On Windows, locking the discovery file would mean it cannot be read by other
/// processes hence the separation from [INI_FILENAME].
const LOCK_FILENAME: &str = "netsim.ini.lock";
const INIT_LOCK_FILENAME: &str = "netsim.ini.init.lock";

/// Parsed configuration from the INI file.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct NetsimConfig {
    pub pid: Option<u32>,
    pub grpc_port: u16,
    pub uds_path: Option<String>,
}

/// Held while initializing. Can only be used to write the INI file once.
#[derive(Debug)]
pub struct IniFileUninitialized {
    ini_file: File,
    init_lock_file: Option<File>,
    lock_file: Option<File>,
    path: PathBuf,
    init_lock_path: PathBuf,
    lock_path: PathBuf,
}

impl IniFileUninitialized {
    pub fn new(
        path: PathBuf,
        lock_path: PathBuf,
        init_lock_path: PathBuf,
        locked_lock_file: File,
    ) -> io::Result<Self> {
        let init_lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&init_lock_path)?;
        init_lock.try_lock()?; // Should succeed as we are owner
        let ini_file = OpenOptions::new().write(true).create(true).truncate(true).open(&path)?;

        Ok(IniFileUninitialized {
            ini_file,
            init_lock_file: Some(init_lock),
            lock_file: Some(locked_lock_file),
            path,
            init_lock_path,
            lock_path,
        })
    }

    pub fn write(mut self, data: &HashMap<String, String>) -> io::Result<IniFileInitialized> {
        let mut writer = BufWriter::new(&mut self.ini_file);
        for (key, value) in data {
            writeln!(writer, "{key}={value}")?;
        }
        writer.flush()?;

        // Init lock should be released but left on disk.
        let _released_init_lock = self.init_lock_file.take();

        Ok(IniFileInitialized {
            path: self.path.clone(),
            lock_path: self.lock_path.clone(),
            lock_file: self.lock_file.take(),
            init_lock_path: self.init_lock_path.clone(),
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Drop for IniFileUninitialized {
    fn drop(&mut self) {
        if let Some(file) = self.init_lock_file.take() {
            drop(file);
            if let Err(err) = fs::remove_file(&self.init_lock_path) {
                if err.kind() != io::ErrorKind::NotFound {
                    warn!("Failed to remove {}: {err}", self.init_lock_path.display());
                }
            }
        }
        if let Some(file) = self.lock_file.take() {
            drop(file);
            if let Err(err) = fs::remove_file(&self.lock_path) {
                if err.kind() != io::ErrorKind::NotFound {
                    warn!("Failed to remove {}: {err}", self.lock_path.display());
                }
            }
        }
    }
}

/// Held for the lifetime of the daemon. It cannot be used to write again.
#[derive(Debug)]
pub struct IniFileInitialized {
    lock_file: Option<File>,
    path: PathBuf,
    lock_path: PathBuf,
    init_lock_path: PathBuf,
}

impl IniFileInitialized {
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Drop for IniFileInitialized {
    fn drop(&mut self) {
        if let Some(file) = self.lock_file.take() {
            drop(file);
            for path in [&self.path, &self.lock_path, &self.init_lock_path] {
                if let Err(err) = fs::remove_file(path) {
                    warn!("Failed to remove {}: {err}", path.display());
                }
            }
        }
    }
}

/// Represents the access level to the INI file.
#[derive(Debug)]
pub enum IniFileAccess {
    Writer(IniFileUninitialized),
    Reader(NetsimConfig),
    Initializing,
}

/// Manages the lifecycle of a lockable INI file used for daemon status and
/// configuration.
pub struct IniFile {
    path: PathBuf,
    lock_path: PathBuf,
    unlocked_lock_file: File,
}

impl IniFile {
    /// Creates a new `IniFile` manager for the default netsim INI file.
    pub fn new() -> io::Result<Self> {
        let dir = get_discovery_directory();
        Self::new_for_dir(dir)
    }

    /// Creates a new `IniFile` manager for an INI file in the specified
    /// directory.
    pub fn new_for_dir(dir: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&dir)?;
        let path = dir.join(INI_FILENAME);
        let lock_path = dir.join(LOCK_FILENAME);
        let unlocked_lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;

        Ok(IniFile { path, lock_path, unlocked_lock_file })
    }

    /// Attempts to acquire the lock and determine the access level.
    pub fn try_acquire(self) -> io::Result<IniFileAccess> {
        let init_lock_path = self.path.with_file_name(INIT_LOCK_FILENAME);
        match self.unlocked_lock_file.try_lock() {
            Ok(()) => Ok(IniFileAccess::Writer(IniFileUninitialized::new(
                self.path,
                self.lock_path,
                init_lock_path,
                self.unlocked_lock_file,
            )?)),
            Err(TryLockError::WouldBlock) => {
                let init_lock_result =
                    OpenOptions::new().read(true).write(true).truncate(false).open(&init_lock_path);

                match init_lock_result {
                    Ok(init_lock) => match init_lock.try_lock() {
                        Ok(()) => {
                            // Initialization done, we can read
                            drop(init_lock);
                            self.read_config().map(IniFileAccess::Reader)
                        }
                        Err(TryLockError::WouldBlock) => Ok(IniFileAccess::Initializing),
                        Err(TryLockError::Error(e)) => Err(e),
                    },
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {
                        // Owner hasn't created the init lock file yet
                        Ok(IniFileAccess::Initializing)
                    }
                    Err(e) => Err(e),
                }
            }
            Err(TryLockError::Error(e)) => Err(e),
        }
    }

    /// Reads the INI file, enforcing a strict key=value format.
    /// Note: This is a simple parser. It does not handle keys or values
    /// containing '=', quotes, or escape sequences. Comments start with '#'
    /// or ';'.
    fn read_shared(&self) -> io::Result<HashMap<String, String>> {
        debug!("read_shared called for {}", self.path.display());
        let mut data = HashMap::new();
        let content = fs::read_to_string(&self.path)?;
        debug!("Content: {:#?}", content);
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            debug!("Line {}: '{}'", line_num + 1, trimmed);
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                debug!("  Skipping comment/empty");
                continue;
            }
            if let Some((key, value)) = trimmed.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                if key.is_empty() {
                    debug!("  Error: Empty key");
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Malformed line {} in INI file: Empty key '{}'",
                            line_num + 1,
                            line
                        ),
                    ));
                }
                debug!("  Parsed: {} = {}", key, value);
                data.insert(key.to_string(), value.to_string());
            } else {
                debug!("  Error: Missing =");
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Malformed line {} in INI file: Missing '=' '{}'", line_num + 1, line),
                ));
            }
        }
        debug!("read_shared success: {:?}", data);
        Ok(data)
    }

    /// Reads and parses the INI file into a NetsimConfig struct.
    fn read_config(&self) -> io::Result<NetsimConfig> {
        let data = self.read_shared()?;
        let mut config = NetsimConfig::default();

        if let Some(pid_str) = data.get("pid") {
            config.pid = match u32::from_str(pid_str) {
                Ok(pid) => Some(pid),
                Err(e) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Invalid value for pid: {} in {}: {}",
                            pid_str,
                            self.path.display(),
                            e
                        ),
                    ));
                }
            };
        }

        let port_str = data.get("grpc.port").ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Missing grpc.port in INI file: {}", self.path.display()),
            )
        })?;
        config.grpc_port = u16::from_str(port_str).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid value for grpc.port: {} in {}: {}",
                    port_str,
                    self.path.display(),
                    e
                ),
            )
        })?;

        if let Some(uds_path_str) = data.get("uds.path") {
            config.uds_path = Some(uds_path_str.to_string());
        }

        Ok(config)
    }

    /// Returns a reference to the path of the INI file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_ini_file_owner_flow() {
        let temp_dir = tempfile::tempdir().unwrap();
        let ini_file = IniFile::new_for_dir(temp_dir.path().to_path_buf()).unwrap();

        match ini_file.try_acquire() {
            Ok(IniFileAccess::Writer(guard)) => {
                let mut data = HashMap::new();
                data.insert("grpc.port".to_string(), "8554".to_string());
                let _initialized_guard = guard.write(&data).unwrap();

                let ini_file2 = IniFile::new_for_dir(temp_dir.path().to_path_buf()).unwrap();
                match ini_file2.try_acquire() {
                    Ok(IniFileAccess::Reader(config)) => {
                        assert_eq!(config.grpc_port, 8554);
                    }
                    _ => panic!("Expected Reader access"),
                }
            }
            _ => panic!("Expected Writer access"),
        }
    }
}
