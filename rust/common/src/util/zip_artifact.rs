//
//  Copyright 2023 Google, Inc.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at:
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

//! # Zip Artifact Class

use std::{
    fs::{read_dir, remove_file, File},
    io::{Read, Result, Write},
    path::PathBuf,
};

use log::warn;
use zip::{result::ZipResult, write::FileOptions, ZipWriter};

use crate::system::netsimd_temp_dir;

use super::time_display::file_current_time;

/// Recurse all files in root and put it in Vec<PathBuf>
fn recurse_files(root: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    // Read all entries in the given root directory
    let entries = read_dir(root)?;
    for entry in entries {
        let entry = entry?;
        let meta = entry.metadata()?;
        // Perform recursion if it's a directory
        if meta.is_dir() {
            let mut subdir = recurse_files(&entry.path())?;
            result.append(&mut subdir);
        }
        if meta.is_file() {
            result.push(entry.path());
        }
    }
    Ok(result)
}

/// Zip the whole netsimd temp directory and store it in temp directory.
pub fn zip_artifacts() -> ZipResult<()> {
    // Fetch all files in netsimd_temp_dir
    let root = netsimd_temp_dir();
    let files = recurse_files(&root)?;

    // Define PathBuf for zip file
    let zip_file = root.join(&format!("netsim_artifacts_{}.zip", file_current_time()));

    // Create a new ZipWriter
    let mut zip_writer = ZipWriter::new(File::create(&zip_file)?);
    let mut buffer = Vec::new();

    // Put each artifact files into zip file
    for file in files {
        let filename = match file.file_name() {
            Some(os_name) => match os_name.to_str() {
                Some(str_name) => {
                    // Avoid zip files
                    if str_name.starts_with("netsim_artifacts") {
                        continue;
                    }
                    str_name
                }
                None => {
                    warn!("Cannot convert {os_name:?} to str");
                    continue;
                }
            },
            None => {
                warn!("Invalid file path for fetching file name {file:?}");
                continue;
            }
        };

        // Write to zip file
        zip_writer.start_file(filename, FileOptions::default())?;
        let mut f = File::open(&file)?;
        f.read_to_end(&mut buffer)?;
        zip_writer.write_all(&buffer)?;
        buffer.clear();

        // Remove the file once written except for log files
        // To preserve the logs after zip, we must keep the log files available.
        if filename != "netsim_stderr.log" && filename != "netsim_stdout.log" {
            remove_file(file)?;
        }
    }

    // Finish writing zip file
    zip_writer.finish()?;
    Ok(())
}
