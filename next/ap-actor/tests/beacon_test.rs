// Copyright 2025-2026 The Android Open Source Project

use crate::world::ApWorld;
use netsim_packets::ethernet::MacAddr;
use netsim_packets::ieee80211::frame::{FrameControl, MacHeader3Addr, SequenceControl};
use zerocopy::IntoBytes;

// ============================================================================
// Feature: Wireless Network Visibility (Beacons)
//
// As a user, I want APs to broadcast beacons so that devices can discover
// the network and view its capabilities (e.g. WiFi 6 support).
// ============================================================================

// Scenario: Create an AP and verify beacons
// Given a new ApWorld
// When an AP is registered with SSID "TestAP"
// Then the World receives a Beacon for "TestAP"
#[tokio::test]
async fn test_create_ap_beacon_generation() {
    log::info!("Scenario: Create an AP and verify beacons");
    let mut world = ApWorld::new().await;

    // When
    world.given_a_registered_ap("TestAP").await;

    // Then
    world.then_beacon_is_received("TestAP").await;
}

// Scenario: Delete AP
// Given a registered AP "DeleteAP"
// When the AP is deleted
// Then no more beacons are received
#[tokio::test]
async fn test_delete_ap_stops_beacons() {
    log::info!("Scenario: Delete AP - stops beacons");
    let mut world = ApWorld::new().await;
    // Given (Implicit via world method logging)
    world.given_a_registered_ap("DeleteAP").await;

    // Ensure we receive at least one beacon first
    world.then_beacon_is_received("DeleteAP").await;

    // When
    world.when_ap_is_deleted().await;

    // Then
    world.then_no_beacons_are_received().await;
}

// Scenario: Create Request for WiFi 6 AP
// Given a new ApWorld
// When a WiFi 6 AP is registered
// Then the World receives a Beacon with HE Capabilities
#[tokio::test]
async fn test_wifi6_beacon() {
    log::info!("Scenario: Create Request for WiFi 6 AP");
    let mut world = ApWorld::new().await;

    // When
    world.given_a_wifi6_ap("WiFi6_AP").await;

    // Verify HE Element
    let rx = world.rx_from_ap.as_mut().expect("AP registered");
    let msg = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("Timeout")
        .expect("Beacon");

    // Verify HE IE (ID 255, Ext 35)
    let mut offset = 36;
    let mut found_he = false;
    while offset < msg.len() {
        if offset + 1 >= msg.len() {
            break;
        }
        let id = msg[offset];
        let len = msg[offset + 1] as usize;
        if offset + 2 + len > msg.len() {
            break;
        }

        if id == 255 {
            if len > 0 && msg[offset + 2] == 35 {
                found_he = true;
                break;
            }
        }
        offset += 2 + len;
    }
    assert!(found_he, "Beacon should contain HE Capabilities IE");
}

// Scenario: Multiple APs
// Given a new ApWorld
// When two APs are registered
// Then the World receives beacons from both
#[tokio::test]
async fn test_create_two_aps() {
    log::info!("Scenario: Multiple APs");
    let mut world = ApWorld::new().await;

    // When
    world.given_a_registered_ap("AP1").await;
    world.given_a_registered_ap("AP2").await;

    // Then
    let rx = world.rx_from_ap.as_mut().expect("APs registered");
    let mut received_ssids = std::collections::HashSet::new();
    let start = std::time::Instant::now();

    while received_ssids.len() < 2 && start.elapsed() < std::time::Duration::from_secs(2) {
        if let Ok(Some(msg)) =
            tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await
        {
            if msg.len() > 38 {
                let len = msg[37] as usize;
                if msg.len() >= 38 + len {
                    let ssid = String::from_utf8_lossy(&msg[38..38 + len]).to_string();
                    received_ssids.insert(ssid);
                }
            }
        }
    }
    assert!(received_ssids.contains("AP1"), "Missing AP1 beacon");
    assert!(received_ssids.contains("AP2"), "Missing AP2 beacon");
}

// Scenario: Active Discovery (Probe Response)
// Given a registered AP
// When a Station sends a Probe Request
// Then the AP responds with a Probe Response
#[tokio::test]
async fn test_probe_response() {
    log::info!("Scenario: Active Discovery (Probe Response) - Generic/Wildcard");
    let mut world = ApWorld::new().await;
    log::info!("Given a registered AP 'ProbeAP'"); // Keep this for now or remove? User said "Given" printed by helper.
                                                   // Wait, given_a_registered_ap prints "Given ...".
    world.given_a_registered_ap("ProbeAP").await;

    // Station sends Probe Req
    let station_mac: MacAddr = "02:00:00:00:00:99".try_into().unwrap();

    // When (Manual send, not helper)
    log::info!("When a Station sends a Wildcard Probe Request");
    let tx = world.tx_to_ap.as_mut().expect("AP registered");
    // FC: Mgmt(00), Probe Req(0100=4) -> 0x40
    let header = MacHeader3Addr::new(
        FrameControl::new(0x0040),
        0,
        MacAddr::BROADCAST, // DA
        station_mac,        // SA
        MacAddr::BROADCAST, // BSSID
        SequenceControl::new(0),
    );

    let mut frame = Vec::new();
    frame.extend_from_slice(header.as_bytes());
    // Body can be empty for our lax parser (Wildcard behavior)

    tx.send(bytes::Bytes::from(frame)).expect("Send Probe Req");

    // Verify Response
    log::info!("Then the AP responds with a Probe Response");
    let rx = world.rx_from_ap.as_mut().expect("AP registered");
    let msg = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("Timeout")
        .expect("Stream closed");

    // Check if it is Probe Response (0x50)
    // 0x50 = Mgmt(00) + Subtype(0101) = 5.
    assert_eq!(msg[0], 0x50);
    // Dest should be Station
    assert_eq!(&msg[4..10], &station_mac.bytes);
    // Source should be AP (BSSID matches default ...01 from given_a_registered_ap)
    assert_eq!(&msg[10..16], &[0x02, 0x00, 0x00, 0x00, 0x00, 0x01]);
}

#[tokio::test]
async fn test_probe_response_ssid_match() {
    log::info!("Scenario: Active Discovery (Probe Response) - Specific SSID Match");
    let mut world = ApWorld::new().await;
    log::info!("Given a registered AP 'MatchAP'");
    world.given_a_registered_ap("MatchAP").await;

    let station_mac: MacAddr = "02:00:00:00:00:99".try_into().unwrap();
    let tx = world.tx_to_ap.as_mut().expect("AP registered");

    log::info!("When a Station sends a Probe Request for 'MatchAP'");
    // FC: Mgmt(00), Probe Req(0100=4) -> 0x40
    let header = MacHeader3Addr::new(
        FrameControl::new(0x0040),
        0,
        MacAddr::BROADCAST, // DA
        station_mac,        // SA
        MacAddr::BROADCAST, // BSSID
        SequenceControl::new(0),
    );

    let mut frame = Vec::new();
    frame.extend_from_slice(header.as_bytes());

    // SSID IE: Tag 0, Len 7, "MatchAP"
    frame.push(0);
    frame.push(7);
    frame.extend_from_slice(b"MatchAP");

    tx.send(bytes::Bytes::from(frame)).expect("Send Probe Req");

    log::info!("Then the AP responds with a Probe Response");
    let rx = world.rx_from_ap.as_mut().expect("AP registered");
    let msg = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("Timeout")
        .expect("Stream closed");
    assert_eq!(msg[0], 0x50);
}

#[tokio::test]
async fn test_probe_response_ssid_mismatch() {
    log::info!("Scenario: Active Discovery - SSID Mismatch");
    let mut world = ApWorld::new().await;
    log::info!("Given a registered AP 'MyAP'");
    world.given_a_registered_ap("MyAP").await;

    let station_mac: MacAddr = "02:00:00:00:00:88".try_into().unwrap();
    let tx = world.tx_to_ap.as_mut().expect("AP registered");

    log::info!("When a Station sends a Probe Request for 'OtherAP'");
    // FC: Mgmt(00), Probe Req(0100=4) -> 0x40
    let header = MacHeader3Addr::new(
        FrameControl::new(0x0040),
        0,
        MacAddr::BROADCAST, // DA
        station_mac,        // SA
        MacAddr::BROADCAST, // BSSID
        SequenceControl::new(0),
    );

    let mut frame = Vec::new();
    frame.extend_from_slice(header.as_bytes());

    // SSID IE: Tag 0, Len 7, "OtherAP"
    frame.push(0);
    frame.push(7);
    frame.extend_from_slice(b"OtherAP");

    tx.send(bytes::Bytes::from(frame)).expect("Send Probe Req");

    log::info!("Then the AP does NOT respond (ignores request)"); // No helper call
    let rx = world.rx_from_ap.as_mut().expect("AP registered");

    // Logic: We might receive Beacons!
    // We need to filter out Beacons (0x80) and ensure NO Probe Resp (0x50) is received.
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_secs(1) {
        if let Ok(Some(msg)) =
            tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await
        {
            if msg[0] == 0x50 {
                panic!("Received Probe Response for mismatched SSID!");
            }
        }
    }
}

#[tokio::test]
async fn test_probe_response_bssid_mismatch() {
    log::info!("Scenario: Active Discovery - BSSID Mismatch");
    let mut world = ApWorld::new().await;
    log::info!("Given a registered AP 'SpecificAP'");
    world.given_a_registered_ap("SpecificAP").await;

    let station_mac: MacAddr = "02:00:00:00:00:77".try_into().unwrap();
    let tx = world.tx_to_ap.as_mut().expect("AP registered");

    // Send Probe Req with Correct SSID but Wrong BSSID (Unicast)
    log::info!("When a Station sends a Probe Request for 'SpecificAP' to wrong BSSID");

    let wrong_bssid: MacAddr = "02:00:00:00:00:99".try_into().unwrap();

    // Probe Req: Type=Management(0), Subtype=ProbeReq(4) -> 0x0040 (LE: 0x40 00)
    let header = MacHeader3Addr::new(
        FrameControl::new(0x0040),
        0,
        wrong_bssid, // DA
        station_mac, // SA
        wrong_bssid, // BSSID
        SequenceControl::new(0),
    );

    let mut frame = Vec::new();
    frame.extend_from_slice(header.as_bytes());

    // SSID IE: Match
    frame.push(0);
    frame.push(10);
    frame.extend_from_slice(b"SpecificAP");

    tx.send(bytes::Bytes::from(frame)).expect("Send Probe Req");

    log::info!("Then the AP does NOT respond");
    let rx = world.rx_from_ap.as_mut().expect("AP registered");

    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_secs(1) {
        if let Ok(Some(msg)) =
            tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await
        {
            if msg[0] == 0x50 {
                panic!("Received Probe Response for mismatched BSSID!");
            }
        }
    }
}
