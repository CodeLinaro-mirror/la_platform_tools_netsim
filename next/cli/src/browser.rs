// Copyright 2022 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// Opening Browser on Linux and MacOS

use std::ffi::OsStr;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use std::process::Command;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn open<T: AsRef<OsStr>>(path: T) {
    let path = path.as_ref();
    println!("Unsupported OS. Open this url:{:?}", path)
}

#[cfg(target_os = "linux")]
pub fn open<T: AsRef<OsStr>>(path: T) {
    let path = path.as_ref();
    let open_handlers = ["xdg-open", "gnome-open", "kde-open"];
    for open_handler in &open_handlers {
        let mut open_cmd = Command::new(open_handler);
        open_cmd.arg(path);
        if open_cmd.output().is_ok() {
            println!("Opened with command : {open_cmd:?}");
            return;
        }
    }
    println!("xdg-open, gnome-open, kde-open not working (linux). Open this url:{path:?}");
}

#[cfg(target_os = "macos")]
pub fn open<T: AsRef<OsStr>>(path: T) {
    let path = path.as_ref();
    let mut open_cmd = Command::new("/usr/bin/open");
    open_cmd.arg(path);
    if open_cmd.output().is_ok() {
        println!("Opened with command : {:?}", open_cmd);
        return;
    }
    println!("/usr/bin/open not working (macos). Open this url:{:?}", path);
}

#[cfg(target_os = "windows")]
pub fn open<T: AsRef<OsStr>>(path: T) {
    let path = path.as_ref();
    let mut cmd_exe = Command::new("cmd.exe");
    let mut explorer = Command::new("explorer");
    let open_cmds = [cmd_exe.args(["/C", "start"]).arg(path), explorer.arg(path)];
    for open_cmd in open_cmds {
        if open_cmd.output().is_ok() {
            println!("Opened with command : {:?}", open_cmd);
            return;
        }
    }
    println!("'start' and 'explorer' command not supported (windows). Open this url:{:?}", path);
}
