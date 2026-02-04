// Copyright 2025 The Android Open Source Project

use crate::world::World;
use bluetooth_actor::beacon_utils::{
    is_le_advertising_report, CMD_LE_SET_EVENT_MASK, CMD_LE_SET_SCAN_ENABLE,
    CMD_LE_SET_SCAN_PARAMS, CMD_SET_EVENT_MASK_STD, REPORT_ADDR_OFFSET, REPORT_NUM_REPORTS_OFFSET,
};
use bytes::Bytes;
use netsim_model::device::Position;
use std::time::Duration;
use tokio;

/// Scenario: Scanner receives RSSI updates from Beacon
///   Given a world with a scanner and a beacon
///   When the scanner enables scanning
///   Then the scanner receives advertising reports with decreasing RSSI as distance increases
#[tokio::test]
async fn test_rssi_updates() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::builder().is_test(true).try_init();

    // GIVEN a world with a scanner and a beacon
    let mut world = World::new();

    // Create Scanner Device using World helper
    // Name "scanner" will be used to reference it.
    world.given_bluetooth_device("scanner").await;

    // Create Beacon Device at initial position (1, 0, 0)
    let beacon_address = "00:00:00:00:00:02";
    world.given_bluetooth_beacon_with_address("beacon", beacon_address).await;

    let beacon_id = *world.chips.get("beacon").expect("Beacon should exist");

    // Move beacon to initial position
    {
        let mut update = netsim_model::chip::ChipUpdate::default();
        update.position = Some(Position { x: 1.0, y: 0.0, z: 0.0 });
        update.id = Some(beacon_id);
        world.client.0.update(beacon_id, update).await?;
    }

    // WHEN the scanner enables scanning
    world.when_packet_sent("scanner", Bytes::from(CMD_SET_EVENT_MASK_STD)).await;
    world.when_packet_sent("scanner", Bytes::from(CMD_LE_SET_EVENT_MASK)).await;
    world.when_packet_sent("scanner", Bytes::from(CMD_LE_SET_SCAN_PARAMS)).await;
    world.when_packet_sent("scanner", Bytes::from(CMD_LE_SET_SCAN_ENABLE)).await;

    // THEN the scanner receives advertising reports with decreasing RSSI as distance increases
    let distances = [1.0, 10.0, 20.0];
    let mut last_rssi = i8::MAX;

    // Derive expected address byte dynamically
    let last_byte_str =
        beacon_address.split(':').last().ok_or("Invalid beacon address format: missing bytes")?;
    let expected_addr_byte = u8::from_str_radix(last_byte_str, 16)?;

    for distance in distances {
        // Move beacon
        {
            let mut update = netsim_model::chip::ChipUpdate::default();
            update.position = Some(Position { x: distance, y: 0.0, z: 0.0 });
            update.id = Some(beacon_id);
            world.client.0.update(beacon_id, update).await?;
        }

        // Wait up to 2 seconds for a valid packet
        let start = std::time::Instant::now();
        let mut found_rssi = None;

        while start.elapsed() < Duration::from_secs(2) {
            let packet = world.receive_packet("scanner").await;

            if is_le_advertising_report(&packet) {
                // Strict validation: Check Num Reports
                if packet[REPORT_NUM_REPORTS_OFFSET] < 1 {
                    continue;
                }

                // Check address.
                if packet.len() > REPORT_ADDR_OFFSET
                    && packet[REPORT_ADDR_OFFSET] == expected_addr_byte
                {
                    // Found our beacon!
                    // RSSI is the last byte
                    if let Some(&rssi_byte) = packet.last() {
                        let rssi = rssi_byte as i8;
                        found_rssi = Some(rssi);
                        break;
                    }
                }
            }
        }

        let rssi = found_rssi.ok_or(format!("Failed to find RSSI for distance {}", distance))?;

        assert!(
            rssi < last_rssi,
            "RSSI {} should be less than last RSSI {} at distance {}",
            rssi,
            last_rssi,
            distance
        );
        last_rssi = rssi;
    }
    Ok(())
}
