// Copyright 2025-2026 The Android Open Source Project

use crate::world::ApWorld;
use netsim_packets::ethernet::MacAddr;
use netsim_packets::ieee80211::{FrameControl, MacHeader3Addr, SequenceControl};
use std::time::Duration;
use zerocopy::IntoBytes;

// Scenario: Station Deauthentication
// Given a registered AP with WPA2 enabled
// And a Station is associated
// When the Station sends a Deauthentication frame
// Then the AP logs "Deauthenticated" (or we verify association requires handshake again)
// For this test, we verify the AP accepts the frame and doesn't crash/error.
#[tokio::test]
async fn test_station_deauth() {
    log::info!("Scenario: Station Deauthentication");
    let mut world = ApWorld::new().await;
    // Config with WPA
    world.given_a_registered_ap_with_wpa("WpaAP", "CorrectPassword").await;

    let station_mac = "02:00:00:00:00:99";
    let station_mac_addr: MacAddr = station_mac.try_into().unwrap();

    // 1. Association Flow
    world.when_station_sends_assoc_req(station_mac).await;
    world.then_station_receives_assoc_resp(station_mac).await;

    // 2. Perform Deauth
    log::info!("When the Station sends a Deauthentication frame");
    let tx = world.tx_to_ap.as_mut().expect("AP registered");

    // Construct Deauth Frame
    let mut frame = Vec::new();
    let bssid: MacAddr = "02:00:00:00:00:01".try_into().unwrap();

    let header = MacHeader3Addr::new(
        FrameControl::new(0x00C0), // Mgmt(00), Deauth(1100) -> 0x00C0
        0,
        bssid,            // DA (AP)
        station_mac_addr, // SA (Station)
        bssid,            // BSSID
        SequenceControl::new(0),
    );
    frame.extend_from_slice(header.as_bytes());

    // Reason Code (2 bytes) - e.g., 0x0001 (Unspecified) or 0x0003 (Leaving ESS)
    // Fixed field for Deauth is just Reason Code.
    let reason_code: u16 = 3;
    frame.extend_from_slice(&reason_code.to_le_bytes());

    tx.send(bytes::Bytes::from(frame)).expect("Failed to send Deauth");

    // 3. Verify System Stability or State Change
    // We check that sending a subsequent encrypted frame (like M2 retry) would NOT be decrypted/accepted/processed as usual,
    // or at least that we don't crash.
    // For now, simple stability check is good.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // TODO: Ideally we should verify shared_keys is empty.
    // Since we cannot inspect AP internal state easily, we rely on logs or side-effects.
    log::info!("Deauth scenario completed without crash.");
}
