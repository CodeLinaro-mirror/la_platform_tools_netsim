// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn test_logging_topics_enabled() {
    // Enable only tcp and udp
    reset_for_testing(Some("tcp,udp"));
    assert!(is_enabled(Topic::Tcp));
    assert!(is_enabled(Topic::Udp));
    assert!(!is_enabled(Topic::Icmp));
    assert!(!is_enabled(Topic::Dhcp));

    // Enable all
    reset_for_testing(Some("all"));
    assert!(is_enabled(Topic::Tcp));
    assert!(is_enabled(Topic::Icmp));
    assert!(is_enabled(Topic::Tftp));

    // Disable all
    reset_for_testing(None);
    assert!(!is_enabled(Topic::Tcp));
    assert!(!is_enabled(Topic::Udp));
}
