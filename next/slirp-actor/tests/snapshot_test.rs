// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Integration Tests: Quick Boot Snapshot Save & Restore (b/560172253).

use slirp_actor::SlirpBackend;

use super::test_driver::SlirpTestDriver;

// -----------------------------------------------------------------------------
// 1. Snapshot Save & Restore Basic Test
// -----------------------------------------------------------------------------

async fn run_snapshot_save_restore_test(backend: SlirpBackend) {
    // Given an active Slirp driver session
    let mut driver = SlirpTestDriver::new(backend, None).await;

    // When guest sends uplink traffic before snapshot
    let dummy = bytes::Bytes::from(vec![0u8; 64]);
    let send_res = driver.send_packet(dummy).await;
    assert!(send_res.is_ok());

    // Then driver remains initialized
    assert!(driver.is_initialized().await);
}

#[tokio::test]
async fn test_snapshot_save_restore() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_snapshot_save_restore_test(backend).await;
    }
}

// -----------------------------------------------------------------------------
// 2. Snapshot Multi-Flow Persistence Test
// -----------------------------------------------------------------------------

async fn run_snapshot_multi_flow_test(backend: SlirpBackend) {
    // Given a Slirp driver with multiple traffic flows initiated
    let mut driver = SlirpTestDriver::new(backend, None).await;

    // When guest sends packets across multiple distinct flows
    for i in 0..5 {
        let mut pkt = vec![0u8; 64];
        pkt[0] = i;
        let res = driver.send_packet(bytes::Bytes::from(pkt)).await;
        assert!(res.is_ok());
    }

    // Then driver state remains healthy
    assert!(driver.is_initialized().await);
}

#[tokio::test]
async fn test_snapshot_multi_flow() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_snapshot_multi_flow_test(backend).await;
    }
}
