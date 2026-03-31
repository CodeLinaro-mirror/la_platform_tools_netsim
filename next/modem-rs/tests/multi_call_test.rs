// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{steps::*, world::World};

// Scenario: Multi Call (Conference/Swap)
//   Given a modem "A"
//   And a modem "B" with number "111"
//   And a modem "C" with number "222"
//   When AT command "ATD111;" is sent to "A"
//   Then wait for connection (OK from A, Ring B, ATA, OK)
//   When AT command "ATD222;" is sent to "A"
//   Then wait for connection (OK from A, Ring C, ATA, OK)
//   When AT command "AT+CHLD=2" is sent to "A"
//   Then response from "A" is "OK"
//   When AT command "AT+CLCC" is sent to "A"
//   Then response from "A" shows "111" is Active and "222" is Held
#[test]
fn test_multi_call_scenario() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    // 1. A calls B
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");

    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK"); // Connected

    // 2. A calls C (B put on hold automatically by logic)
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    // Consume potential extra OK from Hold? (Based on previous findings)
    // I'll use wait_for_response_containing which skips garbage if any.

    then_wait_for_response_containing(&mut world, "C", "RING");

    when_at_command_sent(&mut world, "C", "ATA");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK"); // Connected

    // 3. Swap calls
    when_at_command_sent(&mut world, "A", "AT+CHLD=2");
    then_response_is(&mut world, "A", "OK");

    // 4. Verify states
    // Expected: 111 (Call 1) is Active (State 0)
    //           222 (Call 2) is Held (State 1)
    when_at_command_sent(&mut world, "A", "AT+CLCC");

    // Check 111 is Active (0)
    // Pattern: +CLCC: <idx>,<dir>,0,0,0,"111",...
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,0,\"111\"");

    // Check 222 is Held (1)
    // Pattern: +CLCC: <idx>,<dir>,1,0,0,"222",...
    // Note: Index for 222 is likely 2.
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,1,0,0,\"222\"");

    then_wait_for_response_containing(&mut world, "A", "OK");
}
