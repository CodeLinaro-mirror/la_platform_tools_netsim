// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Configurable debug logging based on SLIRP_DEBUG env var.

use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Topic {
    Tcp = 1 << 0,
    Udp = 1 << 1,
    Icmp = 1 << 2,
    Dhcp = 1 << 3,
    Dns = 1 << 4,
    Tftp = 1 << 5,
    Arp = 1 << 6,
}

static ENABLED_TOPICS: AtomicU32 = AtomicU32::new(0);
static INITIALIZED: std::sync::Once = std::sync::Once::new();

/// Initializes the logging system by reading the SLIRP_DEBUG environment
/// variable. This is called automatically on the first log check.
pub fn init() {
    INITIALIZED.call_once(|| {
        let mut flags = 0;
        if let Ok(val) = std::env::var("SLIRP_DEBUG") {
            for part in val.split(',') {
                match part.trim().to_lowercase().as_str() {
                    "tcp" => flags |= Topic::Tcp as u32,
                    "udp" => flags |= Topic::Udp as u32,
                    "icmp" => flags |= Topic::Icmp as u32,
                    "dhcp" => flags |= Topic::Dhcp as u32,
                    "dns" => flags |= Topic::Dns as u32,
                    "tftp" => flags |= Topic::Tftp as u32,
                    "arp" => flags |= Topic::Arp as u32,
                    "all" => flags = 0xffffffff,
                    _ => {}
                }
            }
        }
        ENABLED_TOPICS.store(flags, Ordering::Relaxed);
    });
}

/// Checks if a specific debug topic is enabled.
pub fn is_enabled(topic: Topic) -> bool {
    init();
    (ENABLED_TOPICS.load(Ordering::Relaxed) & (topic as u32)) != 0
}

#[macro_export]
macro_rules! slirp_debug {
    ($topic:expr, $($arg:tt)+) => {
        if $crate::logging::is_enabled($topic) {
            log::debug!(target: "libslirp", $($arg)+);
        }
    };
}

#[cfg(test)]
pub fn reset_for_testing(val: Option<&str>) {
    init(); // Ensure Once is marked as run!
    let mut flags = 0;
    if let Some(v) = val {
        for part in v.split(',') {
            match part.trim().to_lowercase().as_str() {
                "tcp" => flags |= Topic::Tcp as u32,
                "udp" => flags |= Topic::Udp as u32,
                "icmp" => flags |= Topic::Icmp as u32,
                "dhcp" => flags |= Topic::Dhcp as u32,
                "dns" => flags |= Topic::Dns as u32,
                "tftp" => flags |= Topic::Tftp as u32,
                "arp" => flags |= Topic::Arp as u32,
                "all" => flags = 0xffffffff,
                _ => {}
            }
        }
    }
    ENABLED_TOPICS.store(flags, Ordering::Relaxed);
}

#[cfg(test)]
#[path = "logging/tests.rs"]
mod tests;
