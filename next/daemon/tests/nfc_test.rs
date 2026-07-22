// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use futures::StreamExt;
use netsim_proto::nfc_service::{GetStatusRequest, PollRequest, SendApduRequest, SetPowerRequest};

use crate::world::World;

#[tokio::test]
async fn test_grpc_nfc_service() {
    let mut world = World::new().await;
    world.when_spawn_daemon().await;

    // Test 1: GetStatus for all NFC chips (chip_id = 0)
    let mut get_status_req = GetStatusRequest::new();
    get_status_req.chip_id = 0;
    let get_status_resp = world
        .nfc_client
        .get_status_async(&get_status_req)
        .expect("Failed to call get_status_async")
        .await
        .expect("Failed to get status response");
    assert!(get_status_resp.chips.is_empty());

    // Test 2: SetPower on default fallback chip 0
    let mut set_power_req = SetPowerRequest::new();
    set_power_req.chip_id = 0;
    set_power_req.power_on = false;
    let set_power_resp = world
        .nfc_client
        .set_power_async(&set_power_req)
        .expect("Failed to call set_power_async")
        .await
        .expect("Failed to get set power response");
    let chip = set_power_resp.chip.0.expect("Expected chip in response");
    assert_eq!(chip.id, 0);
    assert_eq!(chip.name, "nfc-default");

    // Test 3: Poll on default fallback chip 0
    let mut poll_req = PollRequest::new();
    poll_req.chip_id = 0;
    let mut poll_stream = world.nfc_client.poll(&poll_req).expect("Failed to call poll");
    let poll_resp = poll_stream
        .next()
        .await
        .expect("Expected poll item")
        .expect("Expected successful poll response");
    assert_eq!(poll_resp.target_id, 1);
    assert_eq!(poll_resp.tag_data, vec![0x04, 0x00]);

    // Test 4: SendApdu on default fallback chip 0
    let mut apdu_req = SendApduRequest::new();
    apdu_req.chip_id = 0;
    apdu_req.apdu = vec![0x00, 0x11, 0x22, 0x33];
    let apdu_resp = world
        .nfc_client
        .send_apdu_async(&apdu_req)
        .expect("Failed to call send_apdu_async")
        .await
        .expect("Failed to get apdu response");
    assert_eq!(apdu_resp.response, vec![0x90, 0x00]);
}
