// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{steps::*, world::World};

// Scenario: Verify PIN Retry Counter
//   Given a modem "A"
//   When AT command 'AT+CPIN="0000"' is sent to "A"
//   Then response from "A" is "ERROR"
//   When AT command 'AT+CPIN="0000"' is sent to "A"
//   Then response from "A" is "ERROR"
//   When AT command "AT+SPIC" is sent to "A"
//   Then response from "A" is "+SPIC: 1"
//   And response from "A" is "OK"
#[test]
fn test_pin_retry_counter() {
    let mut world = World::new();
    given_modem_with_locked_sim(&mut world, "A");

    // First failed attempt
    when_at_command_sent(&mut world, "A", "AT+CPIN=\"0000\"");
    then_response_is(&mut world, "A", "ERROR");

    // Second failed attempt
    when_at_command_sent(&mut world, "A", "AT+CPIN=\"0000\"");
    then_response_is(&mut world, "A", "ERROR");

    // Query retries
    when_at_command_sent(&mut world, "A", "AT+SPIC");
    then_response_is(&mut world, "A", "+SPIC: 1");
    then_response_is(&mut world, "A", "OK");
}
