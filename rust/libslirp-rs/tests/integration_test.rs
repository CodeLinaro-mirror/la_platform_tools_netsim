// Copyright 2024 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use bytes::Bytes;
use libslirp_rs::libslirp::LibSlirp;
use libslirp_rs::libslirp_config::SlirpConfig;

use std::io;
use std::sync::mpsc;
use std::time::Duration;

/// Test whether shutdown closes the rx channel
#[test]
fn it_shutdown() {
    let config = SlirpConfig { ..Default::default() };

    let before_fd_count = count_open_fds().unwrap();

    let (tx, rx) = mpsc::channel::<Bytes>();
    let slirp = LibSlirp::new(config, tx, None, None);
    slirp.shutdown();
    assert_eq!(
        rx.recv_timeout(Duration::from_millis(5)),
        Err(mpsc::RecvTimeoutError::Disconnected)
    );

    let after_fd_count = count_open_fds().unwrap();
    assert_eq!(before_fd_count, after_fd_count);
}

#[cfg(target_os = "linux")]
fn count_open_fds() -> io::Result<usize> {
    use std::fs;
    let entries = fs::read_dir("/proc/self/fd")?;
    Ok(entries.count())
}

#[cfg(not(target_os = "linux"))]
fn count_open_fds() -> io::Result<usize> {
    Ok(0)
}
