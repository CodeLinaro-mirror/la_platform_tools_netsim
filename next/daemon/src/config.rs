// Copyright 2023-2025 The Android Open Source Project

use log::warn;
use named_lock::NamedLock;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

// --- INI File Management ---

/// Manages the lifecycle of a lockable INI file used for daemon status and configuration.
pub struct IniFile {
    path: PathBuf,
    lock: NamedLock,
    _lock_guard: Option<named_lock::NamedLockGuard>,
}

impl IniFile {
    /// Creates a new `IniFile` manager for a file in the specified directory.
    pub fn new(dir: &Path, filename: &str) -> Self {
        let path = dir.join(filename);
        let lock_name = path.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "-");
        let lock = NamedLock::create(&lock_name).unwrap();
        IniFile { path, lock, _lock_guard: None }
    }

    /// Attempts to acquire an exclusive lock file.
    pub fn try_lock(&mut self) -> io::Result<()> {
        match self.lock.try_lock() {
            Ok(guard) => {
                self._lock_guard = Some(guard);
                Ok(())
            }
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "Failed to acquire lock")),
        }
    }

    /// Writes the given `HashMap` to the INI file, overwriting any existing content.
    pub fn write(&self, data: &HashMap<String, String>) -> io::Result<()> {
        if self._lock_guard.is_none() {
            return Err(io::Error::new(io::ErrorKind::Other, "File is not locked for writing"));
        }
        let file = File::create(&self.path)?;
        let mut writer = BufWriter::new(file);
        for (key, value) in data {
            writeln!(writer, "{key}={value}")?;
        }
        writer.flush()?;
        Ok(())
    }

    /// Reads the INI file, enforcing a strict key=value format.
    ///
    /// This method is for clients. It will return an error if it finds any
    /// comments, empty lines, or lines that are not a valid `key=value` pair.
    pub fn read_shared(&self) -> io::Result<HashMap<String, String>> {
        let mut data = HashMap::new();
        let content = fs::read_to_string(&self.path)?;
        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                data.insert(key.trim().to_string(), value.trim().to_string());
            } else {
                // Any line that is not a perfect key=value pair is an error.
                // This includes empty lines and comments.
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Malformed line in INI file: '{}'", line),
                ));
            }
        }
        Ok(data)
    }

    /// Returns a reference to the path of the INI file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Drop for IniFile {
    fn drop(&mut self) {
        if self._lock_guard.is_some() {
            if let Err(e) = fs::remove_file(&self.path) {
                warn!("Failed to remove ini file '{}': {}", self.path.display(), e);
            }
        }
    }
}
