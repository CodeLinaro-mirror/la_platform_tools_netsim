// Copyright 2023-2025 The Android Open Source Project

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

use log::warn;

// --- INI File Management ---

/// Discovery configuration (must remain readable by all instances).
const INI_FILENAME: &str = "netsim.ini";

/// File used for primary instance synchronization.
/// On Windows, locking the discovery file would mean it cannot be read by other
/// processes hence the separation from [INI_FILENAME].
const LOCK_FILENAME: &str = "netsim.ini.lock";

struct DiscoveryDir {
    root_env: &'static str,
    subdir: &'static str,
}

#[cfg(target_os = "linux")]
const DISCOVERY: DiscoveryDir = DiscoveryDir { root_env: "XDG_RUNTIME_DIR", subdir: "" };
#[cfg(target_os = "macos")]
const DISCOVERY: DiscoveryDir =
    DiscoveryDir { root_env: "HOME", subdir: "Library/Caches/TemporaryItems" };
#[cfg(target_os = "windows")]
const DISCOVERY: DiscoveryDir = DiscoveryDir { root_env: "LOCALAPPDATA", subdir: "Temp" };
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("netsim only supports linux, Mac, and Windows");

/// Get discovery directory for netsim
pub fn get_discovery_directory() -> PathBuf {
    // $TMPDIR is the temp directory on buildbots
    if let Ok(test_env_p) = std::env::var("TMPDIR") {
        return PathBuf::from(test_env_p);
    }
    let mut path = match std::env::var(DISCOVERY.root_env) {
        Ok(env_p) => PathBuf::from(env_p),
        Err(_) => {
            warn!("No discovery env for {}, using /tmp", DISCOVERY.root_env);
            PathBuf::from("/tmp")
        }
    };
    path.push(DISCOVERY.subdir);
    path
}

/// Parsed configuration from the INI file.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct NetsimConfig {
    pub pid: Option<u32>,
    pub grpc_port: u16,
    pub uds_path: Option<String>,
}

/// Represents a locked INI file, held by the main daemon instance.
#[derive(Debug)]
pub struct IniFileGuard {
    path: PathBuf,
    lock_path: PathBuf,
    /// Locked handle to [LOCK_FILENAME] that is unlocked on drop.
    _lock_file: File,
}

impl IniFileGuard {
    /// Writes the given `HashMap` to the INI file, overwriting any existing
    /// content.
    pub fn write(&mut self, data: &HashMap<String, String>) -> io::Result<()> {
        let file = File::create(&self.path)?;
        let mut writer = BufWriter::new(file);
        for (key, value) in data {
            writeln!(writer, "{key}={value}")?;
        }
        writer.flush()
    }

    /// Returns a reference to the path of the INI file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Drop for IniFileGuard {
    fn drop(&mut self) {
        // Remove the INI file and lock file as part of cleanup.
        if let Err(err) = fs::remove_file(&self.path) {
            log::warn!("Failed to remove {}: {err}", self.path.display());
        }
        if let Err(err) = fs::remove_file(&self.lock_path) {
            log::warn!("Failed to remove {}: {err}", self.lock_path.display());
        }
    }
}

/// Represents the access level to the INI file.
#[derive(Debug)]
pub enum IniFileAccess {
    /// This instance owns the lock and can write to the file.
    Writer(IniFileGuard),
    /// Another instance owns the lock; this instance can only read.
    Reader(NetsimConfig),
}

/// Manages the lifecycle of a lockable INI file used for daemon status and
/// configuration.
pub struct IniFile {
    path: PathBuf,
    lock_path: PathBuf,
    lock_file: File,
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
        let lock_file = OpenOptions::new().read(true).write(true).create(true).open(&lock_path)?;

        Ok(IniFile { path, lock_path, lock_file })
    }

    /// Attempts to acquire the lock and determine the access level.
    pub fn try_acquire(self) -> io::Result<IniFileAccess> {
        match self.lock_file.try_lock() {
            Ok(()) => Ok(IniFileAccess::Writer(IniFileGuard {
                path: self.path,
                lock_path: self.lock_path,
                _lock_file: self.lock_file,
            })),
            Err(TryLockError::WouldBlock) => {
                // Lock failed, another instance is running.
                warn!(
                    "Failed to acquire lock on {}. Another instance may be running.",
                    self.lock_path.display()
                );
                // TODO(b/487343471): Known race here where we may read an old version of the
                // ini file. We need some sort of "ready" flag to indicate when
                // the primary daemon has written its ini file and it can be
                // read.

                self.read_config().map(IniFileAccess::Reader)
            }
            Err(TryLockError::Error(e)) => Err(e),
        }
    }

    /// Reads the INI file, enforcing a strict key=value format.
    /// Note: This is a simple parser. It does not handle keys or values
    /// containing '=', quotes, or escape sequences. Comments start with '#'
    /// or ';'.
    fn read_shared(&self) -> io::Result<HashMap<String, String>> {
        log::debug!("read_shared called for {}", self.path.display());
        let mut data = HashMap::new();
        let content = fs::read_to_string(&self.path)?;
        log::debug!("Content: {:#?}", content);
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            log::debug!("Line {}: '{}'", line_num + 1, trimmed);
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                log::debug!("  Skipping comment/empty");
                continue;
            }
            if let Some((key, value)) = trimmed.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                if key.is_empty() {
                    log::debug!("  Error: Empty key");
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Malformed line {} in INI file: Empty key '{}'",
                            line_num + 1,
                            line
                        ),
                    ));
                }
                log::debug!("  Parsed: {} = {}", key, value);
                data.insert(key.to_string(), value.to_string());
            } else {
                log::debug!("  Error: Missing =");
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Malformed line {} in INI file: Missing '=' '{}'", line_num + 1, line),
                ));
            }
        }
        log::debug!("read_shared success: {:?}", data);
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
    use std::{collections::HashMap, fs};

    use super::*;

    #[test]
    fn test_ini_file_owner_flow() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut ini_file = IniFile::new_for_dir(temp_dir.path().to_path_buf()).unwrap();

        match ini_file.try_acquire() {
            Ok(IniFileAccess::Writer(mut guard)) => {
                let mut data = HashMap::new();
                data.insert("grpc.port".to_string(), "8554".to_string());
                guard.write(&data).unwrap();

                let mut ini_file2 = IniFile::new_for_dir(temp_dir.path().to_path_buf()).unwrap();
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
