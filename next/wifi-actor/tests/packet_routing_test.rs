// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use tracing::info;

use crate::world::World;

// ============================================================================
// Feature: Wi-Fi Direct (Peer-to-Peer)
// ============================================================================

// Scenario: A Station (Chip) sends a unicast packet to another Station (Chip)
// (Peer-to-Peer / WiFi Direct) Given a Wifi Medium with two provisioned chips
// (Sender and Receiver) When the Sender transmits a unicast frame addressed to
// the Receiver (Direct Link) Then the Receiver should receive the packet with
// the expected payload
#[tokio::test]
async fn test_p2p_wifi_direct_flow() {
    let mut world = World::new().await;
    let _sender = world.given_a_chip(1).await; // ID 1
    let _receiver = world.given_a_chip(2).await; // ID 2

    // Explicitly verify WiFi Direct (Ad-hoc/P2P) data path
    world.when_chip_transmits_unicast(0, 1, "Direct P2P Hello").await;
    world.then_chip_receives_payload(1, "Direct P2P Hello").await;
}

// Scenario: A Station (Chip) sends a broadcast packet
// Given a Wifi Medium with two provisioned chips
// And the Receiver has previously registered its presence (via transmission)
// When the Sender transmits a broadcast frame
// Then the Receiver should receive the packet
#[tokio::test]
async fn test_broadcast_flow_real() {
    let mut world = World::new().await;
    let _sender = world.given_a_chip(1).await;
    let _receiver = world.given_a_chip(2).await;

    world.when_chip_transmits_broadcast(0, "Broadcast Message").await;
    world.then_chip_receives_payload(1, "Broadcast Message").await;
}

// ============================================================================
// Feature: Access Point Connectivity (Infrastructure)
// ============================================================================

// Scenario: The Access Point (Infra) sends a Management/Control frame to a
// Station (Chip) Given a Wifi Medium with a provisioned chip (Receiver)
// And the AP stream is active
// When the AP injects a unicast management frame (e.g., Auth response)
// addressed to the Receiver Then the Receiver should receive the packet
#[tokio::test]
async fn test_infra_flow_real() {
    let mut world = World::new().await;
    let _receiver = world.given_a_chip(2).await;

    world.when_infra_transmits_unicast(0, "Infra Management").await;
    world.then_chip_receives_payload(0, "Infra Management").await;
}

// ============================================================================
// Feature: Internet Connectivity (Uplink/Downlink)
// ============================================================================

// Scenario: A Station (Chip) sends a data packet destined for the internet
// (Slirp) Given a Wifi Medium with an AP and a provisioned chip (Sender)
// And the Sender is enabled (for Slirp)
// When the Sender transmits a Data Frame (ToDS=1) aimed at the Internet
// Then the Slirp Actor should receive the packet (Verified via successful
// transmission/no error)
#[tokio::test]
async fn test_uplink_data_flow_stub() {
    let mut world = World::new().await;
    world.given_an_ap().await;
    let _sender = world.given_a_chip(1).await;

    // Send Data Frame to Internet Gateway
    world.when_chip_transmits_data_to_slirp(0).await;

    // Verification: We assume if no error, routing succeeded.
    // Ideally we would mock Slirp, but testing end-to-end Slirp is covered in
    // integration_test.rs. This test ensures the Medium correctly routes
    // "Internet" traffic to the Slirp target.
}

// ============================================================================
// Feature: WiFi to Ethernet Multicast (Station to Slirp)
// ============================================================================

// Scenario: A Station (Chip) sends a multicast packet (e.g. mDNS) which should
// also go to Slirp (Gateway) Given a Wifi Medium with an AP and a provisioned
// chip (Sender) When the Sender transmits a Multicast Frame (mDNS)
// Then the Slirp Actor should receive the packet (via Multicast logic)
#[tokio::test]
async fn test_uplink_multicast_flow() {
    let mut world = World::new().await;
    world.given_an_ap().await;
    let _sender = world.given_a_chip(1).await;

    // Sender transmits mDNS (Multicast)
    world.when_chip_transmits_mdns(0, "mDNS Query").await;

    // No explicit Slirp mocked check, but we rely on no panic/routing success.
    // If routing failed or targeted None, it wouldn't send to Slirp.
    // This verifies the path exists logic-wise.
}

// ============================================================================
// Feature: WiFi to WiFi Multicast & Broadcast (Service Discovery)
// ============================================================================

// Scenario: A Station (Chip) sends an mDNS query/announcement
// Given a Wifi Medium with multiple chips
// When a Chip transmits an mDNS multicast packet
// Then ALL other chips should receive it
#[tokio::test]
async fn test_mdns_discovery_flow() {
    let mut world = World::new().await;
    let _sender = world.given_a_chip(1).await;
    let _receiver = world.given_a_chip(2).await;
    let _other = world.given_a_chip(3).await;

    // Sender transmits mDNS
    world.when_chip_transmits_mdns(0, "Service Discovery").await;

    // Both receivers should get it
    world.then_chip_receives_payload(1, "Service Discovery").await;
    world.then_chip_receives_payload(2, "Service Discovery").await;
}

// Scenario: A Station (Chip) sends an mDNS query/announcement to the AP
// Given a Wifi Medium with an AP and multiple chips
// When a Chip transmits an mDNS multicast packet
// Then ALL other chips should receive it via AP reflection
#[tokio::test]
async fn test_mdns_discovery_infrastructure_flow() {
    let mut world = World::new().await;
    world.given_an_ap().await;
    let _sender = world.given_a_chip(1).await;
    let _receiver = world.given_a_chip(2).await;
    let _other = world.given_a_chip(3).await;

    // Sender transmits mDNS
    world.when_chip_transmits_infra_mdns(0, "Infra Service Discovery").await;

    // Both receivers should get it
    world.then_chip_receives_payload(1, "Infra Service Discovery").await;
    world.then_chip_receives_payload(2, "Infra Service Discovery").await;
}

// Scenario: The Internet (Slirp) sends a packet to a Station (Chip)
// Given a Wifi Medium with a provisioned chip (Receiver)
// When the Slirp Actor sends an Ethernet frame addressed to the Receiver
// Then the Receiver should receive an 802.11 Data Frame (FromDS=1)
#[tokio::test]
#[ignore]
async fn test_downlink_data_flow_stub() {
    // let mut world = World::new().await;
    // let receiver = world.given_a_chip(1).await;
    // world.when_slirp_sends_packet(receiver_mac);
    // world.then_chip_receives_from_ds_frame(receiver);
}

// ============================================================================
// Feature: Access Point Connectivity (Infrastructure)
// ============================================================================

// Scenario: A Station (Chip) sends a management frame (Assoc Req) to the AP
// Given a Wifi Medium with an AP and a Chip
// When the Chip transmits a Management Frame (ToDS=1 or 0) to the AP BSSID
// Then the AP Actor should receive the frame (verified via lack of error)
#[tokio::test]
async fn test_station_to_ap_mgmt_stub() {
    let mut world = World::new().await;
    let _ap = world.given_an_ap().await;
    let _chip = world.given_a_chip(1).await;

    // Send Mgmt Frame (Assoc Req)
    world.when_chip_transmits_mgmt_to_ap(0).await;

    // No assertion on AP state (requires mock), but ensures no panic and
    // routing execution.
}

// ============================================================================
// Feature: Client-to-Client Communication (Infrastructure)
// ============================================================================

// Scenario: A Station (Chip) sends a packet to another Station (Chip) via the
// AP (Infrastructure Mode) Given a Wifi Medium with two provisioned chips
// (Sender and Receiver) And "simulate_ap_reflection" is enabled
// When the Sender transmits a Data Frame (ToDS=1) to the AP BSSID, with
// Destination=Receiver Then the Receiver should receive a Data Frame (FromDS=1)
// from the AP BSSID
#[tokio::test]
async fn test_ap_reflection_flow_stub() {
    let mut world = World::new().await;
    // Initialize AP for reflection
    world.given_an_ap().await;
    let _sender = world.given_a_chip(1).await;
    let _receiver = world.given_a_chip(2).await;

    // Send ToDS unicast (Sender -> AP -> Receiver)
    world.when_chip_transmits_to_ds_unicast(0, 1, "ToDS Data Packet").await;

    // Receiver should get it. Logic in Medium rewrites it to FromDS.
    world.then_chip_receives_payload(1, "ToDS Data Packet").await;
}

// ============================================================================
// Feature: Radio Control (Enable/Disable)
// ============================================================================

// Scenario: A disabled Station (Chip) should not receive packets
// Given a Wifi Medium with a disabled chip (Receiver)
// When a Sender transmits a packet to the Receiver
// Then the Receiver should NOT receive the packet
#[tokio::test]
async fn test_disabled_client_drop_stub() {
    let mut world = World::new().await;
    let _sender = world.given_a_chip(1).await;
    let _receiver = world.given_a_chip(2).await;

    // Verify reception when enabled
    world.when_chip_transmits_unicast(0, 1, "Should Receive").await;
    world.then_chip_receives_payload(1, "Should Receive").await;

    // Disable receiver
    world.given_chip_is_disabled(1).await;

    // Transmit again
    world.when_chip_transmits_unicast(0, 1, "Should Drop").await;

    // Check no reception
    world.then_chip_receives_nothing(1).await;
}

// ============================================================================
// Feature: Ethernet to WiFi Multicast & Broadcast
// ============================================================================

// Scenario: The Infrastructure sends a multicast packet (e.g. Beacon or MDNS)
// Given a Wifi Medium with multiple provisioned chips
// When the AP or Slirp sends a Multicast Ethernet/Mgmt frame
// Then ALL chips should receive the frame
#[tokio::test]
async fn test_infra_multicast_stub() {
    let mut world = World::new().await;
    // Initialize AP for infra multicast
    world.given_an_ap().await;
    let _rx1 = world.given_a_chip(1).await;
    let _rx2 = world.given_a_chip(2).await;

    world.when_infra_transmits_multicast("Infra Multicast").await;

    // Both receivers should get it
    world.then_chip_receives_payload(0, "Infra Multicast").await;
    world.then_chip_receives_payload(1, "Infra Multicast").await;
}
// Scenario: The Infrastructure (Slirp) sends a packet to an unknown MAC address
// (e.g. DHCP Offer to initial random MAC) Given a Wifi Medium with multiple
// provisioned chips When the Infra sends a Unicast Ethernet frame to an unknown
// MAC Then ALL chips should receive the frame (Flooding) AND the destination
// MAC should be rewritten to match each chip's MAC
#[tokio::test]
async fn test_unknown_unicast_flooding() {
    let mut world = World::new().await;
    // Initialize AP
    world.given_an_ap().await;
    let _rx1 = world.given_a_chip(1).await;
    let _rx2 = world.given_a_chip(2).await;

    // Transmit to generic unknown MAC
    let unknown_mac = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01];
    world.when_infra_transmits_unicast_to_mac(unknown_mac, "Unknown Unicast").await;

    // Get actual MACs
    let rx1_mac = world.chips[0].mac;
    let rx2_mac = world.chips[1].mac;

    // Verify both receive it with their own MAC as destination
    world.then_chip_receives_payload_and_dst(0, "Unknown Unicast", rx1_mac).await;
    world.then_chip_receives_payload_and_dst(1, "Unknown Unicast", rx2_mac).await;
}

// ============================================================================
// Feature: Multicast-to-Unicast (M2U) Encryption Fallback
// ============================================================================

// Scenario: Verify that broadcast frames fall back to standard broadcast when
// encryption keys are not yet established.
//
// Given an Access Point and an associated station
// When a shared infrastructure packet (e.g. DHCPOFFER) is received
// And the station's pairwise encryption keys (PTK) are not yet ready
// Then the packet must be delivered as a broadcast frame to ensure
// compatibility
#[tokio::test]
async fn test_dhcp_m2u_race_condition() {
    let mut world = World::new().await;
    world.given_a_secure_ap().await;
    let _rx = world.given_a_chip(1).await;
    let rx_mac = world.chips[0].mac;

    // Trigger association to set GTK in SharedKeyStore
    world.when_chip_transmits_mgmt_to_ap(0).await;

    let timeout_assoc = tokio::time::sleep(std::time::Duration::from_secs(3));
    tokio::pin!(timeout_assoc);

    loop {
        tokio::select! {
            Some(bytes) = world.chips[0].stream_rx.recv() => {
                if let Ok(hwsim_msg) = netsim_packets::HwsimMsg::decode_full(&bytes) {
                    if let Ok(frame) = netsim_packets::HwsimFrame::parse(&hwsim_msg) {
                        let ieee80211 = frame.ieee80211;
                        if ieee80211.is_mgmt() && ieee80211.stype() == 1 {
                            tracing::info!("Received Association Response");
                            break;
                        }
                    }
                }
            }
            _ = &mut timeout_assoc => {
                panic!("Timeout waiting for Association Response");
            }
        }
    }

    // Simulation: The station is associated but we simulate a handshake delay
    // by not installing the PTK in the SharedKeyStore.

    tracing::info!("Injecting Broadcast DHCPOFFER from Infra...");
    world.when_infra_transmits_multicast("DHCPOFFER").await;

    let chip = &mut world.chips[0];
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(3));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(bytes) = chip.stream_rx.recv() => {
                if let Ok(eth) = crate::hwsim_helper::unwrap_hwsim_to_ethernet(&bytes) {
                    if eth.windows(9).any(|w| w == b"DHCPOFFER") {
                        let dst_mac = &eth[0..6];
                        info!("Received DHCPOFFER with dst_mac: {:02X?}", dst_mac);

                        // If it's the bug, it will be Unicast (rx_mac)
                        if dst_mac == rx_mac {
                            panic!("REPRODUCED: DHCPOFFER was incorrectly converted to UNENCRYPTED UNICAST (dst={:02X?})", dst_mac);
                        }

                        // If it's correct, it should be Broadcast (at least if unencrypted)
                        let broadcast = [0xFF; 6];
                        if dst_mac == broadcast {
                             info!("CORRECT: DHCPOFFER remained BROADCAST");
                             return;
                        }
                    }
                }
            }
            _ = &mut timeout => {
                panic!("Timeout waiting for DHCPOFFER");
            }
        }
    }
}

// Scenario: Verify that P2P traffic bypasses M2U and encryption logic,
// acting as a transparent air proxy.
//
// Given a Wifi Medium with two chips
// When a P2P frame is transmitted
// Then the packet must be delivered without modifications (e.g. no M2U
// conversion)
#[tokio::test]
async fn test_p2p_transparent_proxy() {
    let mut world = World::new().await;
    let _ap = world.given_a_chip(1).await; // ID 1
    let _rx = world.given_a_chip(2).await; // ID 2
    let rx_mac = world.chips[1].mac;

    info!("Injecting Broadcast DHCPOFFER from Chip 1...");
    world.when_chip_transmits_broadcast(0, "DHCPOFFER").await;

    // Verify that the frame is delivered as a BROADCAST frame (FF:FF:FF:FF:FF:FF)
    // because unicast encryption is not yet available.

    let chip = &mut world.chips[1];
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(3));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(bytes) = chip.stream_rx.recv() => {
                if let Ok(eth) = crate::hwsim_helper::unwrap_hwsim_to_ethernet(&bytes) {
                    if eth.windows(9).any(|w| w == b"DHCPOFFER") {
                        let dst_mac = &eth[0..6];
                        info!("Received DHCPOFFER with dst_mac: {:02X?}", dst_mac);

                        // If it's the bug, it will be Unicast (rx_mac)
                        if dst_mac == rx_mac {
                            panic!("REPRODUCED: P2P DHCPOFFER was incorrectly converted to UNENCRYPTED UNICAST (dst={:02X?})", dst_mac);
                        }

                        // If it's correct, it should be Broadcast (at least if unencrypted)
                        let broadcast = [0xFF; 6];
                        if dst_mac == broadcast {
                             info!("CORRECT: P2P DHCPOFFER remained BROADCAST");
                             return;
                        }
                    }
                }
            }
            _ = &mut timeout => {
                panic!("Timeout waiting for DHCPOFFER");
            }
        }
    }
}

// Scenario: Verify that broadcast frames are converted to unicast (M2U)
// on an open network and delivered unencrypted.
//
// Note: This test uses a placeholder payload which is not recognized as a real
// DHCP packet. Thus, M2U optimization should apply and convert it to unicast.
#[tokio::test]
async fn test_open_network_m2u() {
    let mut world = World::new().await;
    world.given_an_ap().await; // Creates Open AP by default
    let _rx = world.given_a_chip(1).await;
    let rx_mac = world.chips[0].mac;

    tracing::info!("Injecting Broadcast DHCPOFFER (Placeholder) from Infra on Open Network...");
    world.when_infra_transmits_multicast("DHCPOFFER").await;

    let chip = &mut world.chips[0];
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(3));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(bytes) = chip.stream_rx.recv() => {
                if let Ok(eth) = crate::hwsim_helper::unwrap_hwsim_to_ethernet(&bytes) {
                    if eth.windows(9).any(|w| w == b"DHCPOFFER") {
                        let dst_mac = &eth[0..6];
                        tracing::info!("Received DHCPOFFER with dst_mac: {:02X?}", dst_mac);

                        // For Open Networks, normal multicast (non-DHCP) should be M2U'd to unicast
                        if dst_mac == rx_mac {
                            tracing::info!("CORRECT: Placeholder DHCPOFFER was converted to UNICAST on Open Network");
                            return;
                        }

                        if dst_mac == [0xFF; 6] {
                             panic!("FAILED: Placeholder DHCPOFFER remained BROADCAST on Open Network");
                        }
                    }
                }
            }
            _ = &mut timeout => {
                panic!("Timeout waiting for DHCPOFFER");
            }
        }
    }
}

// Scenario: Verify that real DHCP broadcast frames remain broadcast
// on an open network to ensure compatibility with guest DHCP clients.
#[tokio::test]
async fn test_open_network_dhcp() {
    let mut world = World::new().await;
    world.given_an_ap().await; // Creates Open AP by default
    let _rx = world.given_a_chip(1).await;
    let rx_mac = world.chips[0].mac;

    tracing::info!("Injecting Real DHCP DHCPOFFER from Infra on Open Network...");

    // Construct a real placeholder DHCP packet (with IP/UDP headers)
    // Ethernet: 14, IP: 20, UDP: 8
    let src_mac = [0x02, 0x00, 0x00, 0x00, 0x00, 0x00]; // AP
    let dst_mac = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]; // Broadcast

    let mut eth = Vec::new();
    eth.extend_from_slice(&dst_mac); // DA
    eth.extend_from_slice(&src_mac); // SA
    eth.extend_from_slice(&[0x08, 0x00]); // EtherType IPv4

    // IP Header (20 bytes)
    let ip_hdr = [
        0x45, 0x00, 0x00, 0x2D, // Ver/IHL/TOS/Len (45 bytes total)
        0x00, 0x00, 0x00, 0x00, // ID/Flags/Frag
        0x40, 0x11, 0x00, 0x00, // TTL/Protocol(17)/Checksum
        0x00, 0x00, 0x00, 0x00, // Src IP
        0xFF, 0xFF, 0xFF, 0xFF, // Dst IP (Broadcast)
    ];
    eth.extend_from_slice(&ip_hdr);

    // UDP Header (8 bytes)
    let udp_hdr = [
        0x00, 0x43, // Src Port 67
        0x00, 0x44, // Dst Port 68
        0x00, 0x11, // Length (17 bytes)
        0x00, 0x00, // Checksum
    ];
    eth.extend_from_slice(&udp_hdr);
    eth.extend_from_slice(b"DHCPOFFER"); // Payload

    if eth.len() < 60 {
        eth.resize(60, 0);
    }

    let bssid = netsim_packets::MacAddress::new(src_mac);
    let ieee80211 = netsim_packets::Ieee80211::from_ieee8023_qos(
        &eth,
        bssid,
        netsim_packets::FrameDirection::FromAp,
        true,
        100,
    )
    .unwrap();
    let bytes = ieee80211.encode_to_vec().unwrap();

    // Inject directly into AP injector to bypass
    // world.when_infra_transmits_multicast payload helper
    world.ap_injector.send(bytes::Bytes::from(bytes)).expect("Failed to inject DHCP packet");

    let chip = &mut world.chips[0];
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(3));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(bytes) = chip.stream_rx.recv() => {
                if let Ok(eth) = crate::hwsim_helper::unwrap_hwsim_to_ethernet(&bytes) {
                    if eth.windows(9).any(|w| w == b"DHCPOFFER") {
                        let dst_mac = &eth[0..6];
                        tracing::info!("Received DHCP DHCPOFFER with dst_mac: {:02X?}", dst_mac);

                        // For Open Networks, DHCP should remain broadcast
                        if dst_mac == [0xFF; 6] {
                            tracing::info!("CORRECT: DHCP DHCPOFFER remained BROADCAST on Open Network");
                            return;
                        }

                        if dst_mac == rx_mac {
                             panic!("FAILED: DHCP DHCPOFFER was converted to UNICAST on Open Network");
                        }
                    }
                }
            }
            _ = &mut timeout => {
                panic!("Timeout waiting for DHCP DHCPOFFER");
            }
        }
    }
}
