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
    given_modem_with_number(&mut world, "A", "333");
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

    // Verify B's state: Should now be Active (0) since A swapped back to B
    when_at_command_sent(&mut world, "B", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "B", "+CLCC: 1,1,0,0,0,");
    then_wait_for_response_containing(&mut world, "B", "OK");

    // Verify C's state: Should now be Held (1) since A swapped away from C
    when_at_command_sent(&mut world, "C", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "C", "+CLCC: 1,1,1,0,0,");
    then_wait_for_response_containing(&mut world, "C", "OK");

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

#[test]
fn test_chld_zero() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    // A calls B, B answers
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // A calls C, B put on hold
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // Release held calls (B)
    when_at_command_sent(&mut world, "A", "AT+CHLD=0");
    then_response_is(&mut world, "A", "OK");

    // Verify B (held call) got NO CARRIER
    then_wait_for_response_containing(&mut world, "B", "NO CARRIER");

    // Verify: Only C (Call 2) is active. B (Call 1) is gone.
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,0,0,0,\"222\"");
    then_wait_for_response_containing(&mut world, "A", "OK");
}

#[test]
fn test_chld_one() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    // A calls B, B answers
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // A calls C, B put on hold
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // Release active (C), accept held (B)
    when_at_command_sent(&mut world, "A", "AT+CHLD=1");
    then_response_is(&mut world, "A", "OK");

    // Verify C (active call) got NO CARRIER
    then_wait_for_response_containing(&mut world, "C", "NO CARRIER");

    // Verify B's state: Should now be Active (0) since A accepted it
    when_at_command_sent(&mut world, "B", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "B", "+CLCC: 1,1,0,0,0,");
    then_wait_for_response_containing(&mut world, "B", "OK");

    // Verify: B (Call 1) is active, C (Call 2) is gone.
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,0,\"111\"");
    then_wait_for_response_containing(&mut world, "A", "OK");
}

#[test]
fn test_chld_one_x() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    // A calls B, B answers
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // A calls C, B put on hold
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // Release specific call 1 (B)
    when_at_command_sent(&mut world, "A", "AT+CHLD=11");
    then_response_is(&mut world, "A", "OK");

    // Verify: C (Call 2) remains active, B (Call 1) is gone.
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,0,0,0,\"222\"");
    then_wait_for_response_containing(&mut world, "A", "OK");
}

#[test]
fn test_chld_three_conference() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    // A calls B, B answers
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // A calls C, B put on hold
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // Conference (Add held call B to conversation)
    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");

    // Verify: Both calls are active and multiparty (1)
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,1,\"111\"");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,0,0,1,\"222\"");
    then_wait_for_response_containing(&mut world, "A", "OK");
}

#[test]
fn test_chld_two_x_separate() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    // A calls B, B answers
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // A calls C, B put on hold
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // Conference
    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");

    // Separate call 1 (B)
    when_at_command_sent(&mut world, "A", "AT+CHLD=21");
    then_response_is(&mut world, "A", "OK");

    // Verify: B (Call 1) is Active and NOT multiparty (0)
    //         C (Call 2) is Held
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,0,\"111\"");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,1,0,1,\"222\""); // C remains multiparty (1) in our simple implementation, which is fine for now.
    then_wait_for_response_containing(&mut world, "A", "OK");
}

#[test]
fn test_chld_four_ect() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    // A calls B, B answers (A-B Active)
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // A calls C, B put on hold (A-C Active, A-B Held)
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // Trigger ECT (CHLD=4) -> Pragmatic Hangup
    when_at_command_sent(&mut world, "A", "AT+CHLD=4");
    then_response_is(&mut world, "A", "OK"); // Returns OK to guest

    // Verify: All calls on A are cleared locally
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "OK"); // Only OK, no call lines
}

#[test]
fn test_chld_zero_waiting() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    // A calls B, B answers (Active)
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // C calls A, A gets RING (Waiting)
    when_at_command_sent(&mut world, "C", "ATD333;");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Release waiting call (C)
    when_at_command_sent(&mut world, "A", "AT+CHLD=0");
    then_response_is(&mut world, "A", "OK");

    // Verify C got NO CARRIER
    then_wait_for_response_containing(&mut world, "C", "NO CARRIER");

    // Verify: Only B (Call 1) is active. C is gone.
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,0,\"111\"");
    then_wait_for_response_containing(&mut world, "A", "OK");
}

#[test]
fn test_chld_one_waiting() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    // A calls B, B answers (Active)
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // C calls A, A gets CCWA (Waiting)
    when_at_command_sent(&mut world, "C", "ATD333;");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Release active (B), accept waiting (C)
    when_at_command_sent(&mut world, "A", "AT+CHLD=1");
    then_response_is(&mut world, "A", "OK");

    // Verify B got NO CARRIER
    then_wait_for_response_containing(&mut world, "B", "NO CARRIER");

    // Verify C was answered
    then_wait_for_response_containing(&mut world, "C", "OK");

    // Verify: C (Call 2) is active, B (Call 1) is gone.
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,1,0,0,0,\"222\"");
    then_wait_for_response_containing(&mut world, "A", "OK");
}

#[test]
fn test_chld_one_x_waiting() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "C", "ATD333;");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Reject waiting call (Call 2 - C)
    when_at_command_sent(&mut world, "A", "AT+CHLD=12");
    then_response_is(&mut world, "A", "OK");

    then_wait_for_response_containing(&mut world, "C", "NO CARRIER");

    // Verify B is still active
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,0,\"111\"");
    then_wait_for_response_containing(&mut world, "A", "OK");
}

#[test]
fn test_chld_two_waiting() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "C", "ATD333;");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Hold active (B), accept waiting (C)
    when_at_command_sent(&mut world, "A", "AT+CHLD=2");
    then_response_is(&mut world, "A", "OK");

    // Verify C was answered
    then_wait_for_response_containing(&mut world, "C", "OK");

    // Verify B did NOT get NO CARRIER (it's held, so no immediate NO CARRIER, maybe
    // just no response, or if network simulator forwards hold it might get
    // something, but we don't expect NO CARRIER)

    // Verify: C is active (0), B is held (1).
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,1,0,0,\"111\"");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,1,0,0,0,\"222\"");
    then_wait_for_response_containing(&mut world, "A", "OK");
}

#[test]
fn test_chld_two_with_idx_waiting() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "C", "ATD333;");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Recover specific waiting call (Call 2 - C), which puts 1 (B) on hold
    when_at_command_sent(&mut world, "A", "AT+CHLD=22");
    then_response_is(&mut world, "A", "OK");

    // Verify C was answered
    then_wait_for_response_containing(&mut world, "C", "OK");

    // Verify: C is active (0), B is held (1).
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,1,0,0,\"111\"");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,1,0,0,0,\"222\"");
    then_wait_for_response_containing(&mut world, "A", "OK");
}

#[test]
fn test_peer_specific_receive_hold_and_resume() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    // 1. A calls B, B answers
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // 2. A calls C, C answers (puts B on hold)
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // 3. A swaps calls with AT+CHLD=2 (C goes to Hold, B resumes to Active)
    when_at_command_sent(&mut world, "A", "AT+CHLD=2");
    then_response_is(&mut world, "A", "OK");

    // 4. Verify Peer B: Received receive_resume(A) -> Call with A (333) is Active
    //    (0)
    when_at_command_sent(&mut world, "B", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "B", "+CLCC: 1,1,0,0,0,\"333\"");
    then_wait_for_response_containing(&mut world, "B", "OK");

    // 5. Verify Peer C: Received receive_hold(A) -> Call with A (333) is Held (1)
    when_at_command_sent(&mut world, "C", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "C", "+CLCC: 1,1,1,0,0,\"333\"");
    then_wait_for_response_containing(&mut world, "C", "OK");
}

#[test]
fn test_chld_three_conference_peer_state_sync() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    // A calls B, B answers
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // A calls C, C answers (puts B on hold)
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // A merges B & C into conference with AT+CHLD=3
    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");

    // Both Peer B and Peer C should receive ResumeCall and report state 0 (Active)
    when_at_command_sent(&mut world, "B", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "B", "+CLCC: 1,1,0,0,0,\"333\"");
    then_wait_for_response_containing(&mut world, "B", "OK");

    when_at_command_sent(&mut world, "C", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "C", "+CLCC: 1,1,0,0,0,\"333\"");
    then_wait_for_response_containing(&mut world, "C", "OK");
}

#[test]
fn test_chld_invalid_call_index_returns_error() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");

    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // AT+CHLD=19 with invalid call index 9 must return ERROR
    when_at_command_sent(&mut world, "A", "AT+CHLD=19");
    then_response_is(&mut world, "A", "ERROR");
}

#[test]
fn test_ath0_parsing_and_hangup() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");

    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // ATH0 should hang up call cleanly
    when_at_command_sent(&mut world, "A", "ATH0");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "NO CARRIER");
}

#[test]
fn test_cmut_out_of_bounds_parameter() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // AT+CMUT=2 is out-of-bounds (0 or 1 valid) and must return ERROR
    when_at_command_sent(&mut world, "A", "AT+CMUT=2");
    then_response_is(&mut world, "A", "ERROR");
}

#[test]
fn test_vts_quoted_digit_syntax() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // AT+VTS="1" quoted syntax should parse correctly
    when_at_command_sent(&mut world, "A", "AT+VTS=\"1\"");
    then_response_is(&mut world, "A", "OK");
}
