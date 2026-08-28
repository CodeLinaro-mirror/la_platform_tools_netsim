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
    then_wait_for_response_containing(&mut world, "A", "RING"); // Connected

    // 2. A calls C (B put on hold automatically by logic)
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");

    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING"); // Connected

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
    // Pattern: +CLCC: <idx>,<dir>,0,0,0,111,...
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,0,111");

    // Check 222 is Held (1)
    // Pattern: +CLCC: <idx>,<dir>,1,0,0,222,...
    // Note: Index for 222 is likely 2.
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,1,0,0,222");

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
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A calls C, B put on hold
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Release held calls (B)
    when_at_command_sent(&mut world, "A", "AT+CHLD=0");
    then_response_is(&mut world, "A", "OK");

    // Verify B (held call) got RING
    then_wait_for_response_containing(&mut world, "B", "RING");

    // Verify: Only C (Call 2) is active. B (Call 1) is gone.
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,0,0,0,222");
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
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A calls C, B put on hold
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Release active (C), accept held (B)
    when_at_command_sent(&mut world, "A", "AT+CHLD=1");
    then_response_is(&mut world, "A", "OK");

    // Verify C (active call) got RING
    then_wait_for_response_containing(&mut world, "C", "RING");

    // Verify B's state: Should now be Active (0) since A accepted it
    when_at_command_sent(&mut world, "B", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "B", "+CLCC: 1,1,0,0,0,");
    then_wait_for_response_containing(&mut world, "B", "OK");

    // Verify: B (Call 1) is active, C (Call 2) is gone.
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,0,111");
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
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A calls C, B put on hold
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Release specific call 1 (B)
    when_at_command_sent(&mut world, "A", "AT+CHLD=11");
    then_response_is(&mut world, "A", "OK");

    // Verify: C (Call 2) remains active, B (Call 1) is gone.
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,0,0,0,222");
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
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A calls C, B put on hold
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Conference (Add held call B to conversation)
    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");

    // Verify: Both calls are active and multiparty (1)
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,1,111");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,0,0,1,222");
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
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A calls C, B put on hold
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Conference
    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");

    // Separate call 1 (B)
    when_at_command_sent(&mut world, "A", "AT+CHLD=21");
    then_response_is(&mut world, "A", "OK");

    // Verify: B (Call 1) is Active and NOT multiparty (0)
    //         C (Call 2) is Held and NOT multiparty (0) (degraded because single
    // held)
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,0,111");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,1,0,0,222");
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
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A calls C, B put on hold (A-C Active, A-B Held)
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

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
    then_wait_for_response_containing(&mut world, "A", "RING");

    // C calls A, A gets RING (Waiting)
    when_at_command_sent(&mut world, "C", "ATD333;");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Release waiting call (C)
    when_at_command_sent(&mut world, "A", "AT+CHLD=0");
    then_response_is(&mut world, "A", "OK");

    // Verify C got RING
    then_wait_for_response_containing(&mut world, "C", "RING");

    // Verify: Only B (Call 1) is active. C is gone.
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,0,111");
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
    then_wait_for_response_containing(&mut world, "A", "RING");

    // C calls A, A gets CCWA (Waiting)
    when_at_command_sent(&mut world, "C", "ATD333;");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Release active (B), accept waiting (C)
    when_at_command_sent(&mut world, "A", "AT+CHLD=1");
    then_response_is(&mut world, "A", "OK");

    // Verify B got RING
    then_wait_for_response_containing(&mut world, "B", "RING");

    // Verify C was answered
    then_wait_for_response_containing(&mut world, "C", "RING");

    // Verify: C (Call 2) is active, B (Call 1) is gone.
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,1,0,0,0,222");
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
    then_wait_for_response_containing(&mut world, "A", "RING");

    when_at_command_sent(&mut world, "C", "ATD333;");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Reject waiting call (Call 2 - C)
    when_at_command_sent(&mut world, "A", "AT+CHLD=12");
    then_response_is(&mut world, "A", "OK");

    then_wait_for_response_containing(&mut world, "C", "RING");

    // Verify B is still active
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,0,111");
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
    then_wait_for_response_containing(&mut world, "A", "RING");

    when_at_command_sent(&mut world, "C", "ATD333;");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Hold active (B), accept waiting (C)
    when_at_command_sent(&mut world, "A", "AT+CHLD=2");
    then_response_is(&mut world, "A", "OK");

    // Verify C was answered
    then_wait_for_response_containing(&mut world, "C", "RING");

    // Verify: C is active (0), B is held (1).
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,1,0,0,111");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,1,0,0,0,222");
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
    then_wait_for_response_containing(&mut world, "A", "RING");

    when_at_command_sent(&mut world, "C", "ATD333;");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Recover specific waiting call (Call 2 - C), which puts 1 (B) on hold
    when_at_command_sent(&mut world, "A", "AT+CHLD=22");
    then_response_is(&mut world, "A", "OK");

    // Verify C was answered
    then_wait_for_response_containing(&mut world, "C", "RING");

    // Verify: C is active (0), B is held (1).
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,1,0,0,111");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,1,0,0,0,222");
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
    then_wait_for_response_containing(&mut world, "A", "RING");

    // 2. A calls C, C answers (puts B on hold)
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // 3. A swaps calls with AT+CHLD=2 (C goes to Hold, B resumes to Active)
    when_at_command_sent(&mut world, "A", "AT+CHLD=2");
    then_response_is(&mut world, "A", "OK");

    // 4. Verify Peer B: Received receive_resume(A) -> Call with A (333) is Active
    //    (0)
    when_at_command_sent(&mut world, "B", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "B", "+CLCC: 1,1,0,0,0,333");
    then_wait_for_response_containing(&mut world, "B", "OK");

    // 5. Verify Peer C: Received receive_hold(A) -> Call with A (333) is Held (1)
    when_at_command_sent(&mut world, "C", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "C", "+CLCC: 1,1,1,0,0,333");
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
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A calls C, C answers (puts B on hold)
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A merges B & C into conference with AT+CHLD=3
    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");

    // Both Peer B and Peer C should receive ResumeCall and report state 0 (Active)
    when_at_command_sent(&mut world, "B", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "B", "+CLCC: 1,1,0,0,0,333");
    then_wait_for_response_containing(&mut world, "B", "OK");

    when_at_command_sent(&mut world, "C", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "C", "+CLCC: 1,1,0,0,0,333");
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
    then_wait_for_response_containing(&mut world, "A", "RING");

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
    then_wait_for_response_containing(&mut world, "A", "RING");

    // ATH0 should hang up call cleanly
    when_at_command_sent(&mut world, "A", "ATH0");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
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

#[test]
fn test_outbound_call_cancelled_before_answer_ath() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");

    // A dials B
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");

    // Before B answers, A cancels via ATH
    when_at_command_sent(&mut world, "A", "ATH");
    then_response_is(&mut world, "A", "OK");

    // B should receive remote hangup URC (RING)
    then_wait_for_response_containing(&mut world, "B", "RING");

    // Verify both modems are now idle
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "B", "AT+CLCC");
    then_response_is(&mut world, "B", "OK");
}

#[test]
fn test_outbound_call_cancelled_before_answer_chld() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");

    // A dials B
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");

    // Before B answers, A terminates the dialing call 1 via AT+CHLD=11
    when_at_command_sent(&mut world, "A", "AT+CHLD=11");
    then_response_is(&mut world, "A", "OK");

    // B should receive remote hangup URC (RING)
    then_wait_for_response_containing(&mut world, "B", "RING");

    // Verify both modems are now idle
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "B", "AT+CLCC");
    then_response_is(&mut world, "B", "OK");
}

#[test]
fn test_chld_one_x_selective_hangup_in_conference() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    // A calls B, B answers
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A calls C, B put on hold, C answers
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Merge calls into multi-party conference (AT+CHLD=3)
    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");

    // Selectively hang up call 1 (B) within conference using AT+CHLD=11
    when_at_command_sent(&mut world, "A", "AT+CHLD=11");
    then_response_is(&mut world, "A", "OK");

    // Verify B received remote hangup URC
    then_wait_for_response_containing(&mut world, "B", "RING");

    // Verify: C (Call 2) remains active on A with is_multi_party degraded to false
    // (0), B (Call 1) is gone
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,0,0,0,222");
    then_wait_for_response_containing(&mut world, "A", "OK");
}

#[test]
fn test_simultaneous_double_hangup_race() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");

    // A calls B, B answers
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A initiates hangup with ATH
    when_at_command_sent(&mut world, "A", "ATH");
    then_response_is(&mut world, "A", "OK");

    // B receives the asynchronous peer hangup notification URC
    then_wait_for_response_containing(&mut world, "B", "RING");

    // B attempts teardown via AT+CHLD=1 (idempotent OK on idle state)
    when_at_command_sent(&mut world, "B", "AT+CHLD=1");
    then_response_is(&mut world, "B", "OK");

    // Verify both modems are cleanly idle without panics or leaked state
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "B", "AT+CLCC");
    then_response_is(&mut world, "B", "OK");
}

#[test]
fn test_chld_two_x_separate_from_4way_conference() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");
    given_modem_with_number(&mut world, "D", "444");

    // A calls B, B answers
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A calls C, C answers (puts B on hold)
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A merges B & C into conference
    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");

    // A calls D, D answers (puts B & C conference on hold)
    when_at_command_sent(&mut world, "A", "ATD444;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "D", "RING");
    when_at_command_sent(&mut world, "D", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A merges D into the conference -> 3 remote participants in Active conference
    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");

    // A separates Call 1 (B) via AT+CHLD=21:
    // Call 1 becomes Active (mpty=0)
    // Calls 2 (C) and 3 (D) become Held, and retain mpty=1 (held conference of 2
    // peers)
    when_at_command_sent(&mut world, "A", "AT+CHLD=21");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,0,111");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,1,0,1,222");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 3,0,1,0,1,444");
    then_wait_for_response_containing(&mut world, "A", "OK");
}

#[test]
fn test_concurrent_two_pair_dialing_no_crosstalk() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "111");
    given_modem_with_number(&mut world, "B", "222");
    given_modem_with_number(&mut world, "C", "333");
    given_modem_with_number(&mut world, "D", "444");

    // Pair 1: A dials B (222)
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");

    // Pair 2: C dials D (444) simultaneously
    when_at_command_sent(&mut world, "C", "ATD444;");
    then_response_is(&mut world, "C", "OK");
    then_wait_for_response_containing(&mut world, "D", "RING");

    // Callee D (Pair 2) answers first with ATA
    when_at_command_sent(&mut world, "D", "ATA");
    then_wait_for_response_containing(&mut world, "C", "RING");

    // Callee B (Pair 1) answers second with ATA
    when_at_command_sent(&mut world, "B", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Verify Pair 1: A is connected to B (222), B is connected to A (111)
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,0,222");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "B", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "B", "+CLCC: 1,1,0,0,0,111");
    then_wait_for_response_containing(&mut world, "B", "OK");

    // Verify Pair 2: C is connected to D (444), D is connected to C (333)
    when_at_command_sent(&mut world, "C", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "C", "+CLCC: 1,0,0,0,0,444");
    then_wait_for_response_containing(&mut world, "C", "OK");

    when_at_command_sent(&mut world, "D", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "D", "+CLCC: 1,1,0,0,0,333");
    then_wait_for_response_containing(&mut world, "D", "OK");
}

#[test]
fn test_chld_two_swap_entire_conference_group() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");
    given_modem_with_number(&mut world, "D", "444");

    // A calls B, B answers
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A calls C, C answers (puts B on hold)
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A merges B & C into Active conference (AT+CHLD=3)
    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");

    // D calls A while A is in conference -> arrives as Waiting
    when_at_command_sent(&mut world, "D", "ATD333;");
    then_response_is(&mut world, "D", "OK");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A sends AT+CHLD=2:
    // Entire conference {B, C} goes to Held (mpty=1)
    // Call 3 (D) is answered and becomes Active (mpty=0)
    when_at_command_sent(&mut world, "A", "AT+CHLD=2");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "D", "RING");

    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,1,0,1,111");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,1,0,1,222");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 3,1,0,0,0,444");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // A sends AT+CHLD=2 again:
    // Call 3 (D) goes to Held (mpty=0)
    // Entire conference {B, C} resumes to Active (mpty=1)
    when_at_command_sent(&mut world, "A", "AT+CHLD=2");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,1,111");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,0,0,1,222");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 3,1,1,0,0,444");
    then_wait_for_response_containing(&mut world, "A", "OK");
}

#[test]
fn test_remote_peer_hangup_in_active_conference() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    // A calls B, B answers
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A calls C, C answers (puts B on hold)
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // A merges B & C into conference
    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");

    // Remote participant B hangs up with ATH
    when_at_command_sent(&mut world, "B", "ATH");
    then_response_is(&mut world, "B", "OK");

    // Host A receives remote hangup URC
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Verify: Call 1 (B) is removed, and Call 2 (C) automatically degraded to
    // mpty=0
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,0,0,0,222");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // Verify C is still active with A
    when_at_command_sent(&mut world, "C", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "C", "+CLCC: 1,1,0,0,0,333");
    then_wait_for_response_containing(&mut world, "C", "OK");
}

#[test]
fn test_two_independent_concurrent_conferences() {
    let mut world = World::new();
    // Group 1
    given_modem_with_number(&mut world, "A", "111");
    given_modem_with_number(&mut world, "B", "222");
    given_modem_with_number(&mut world, "C", "333");
    // Group 2
    given_modem_with_number(&mut world, "D", "444");
    given_modem_with_number(&mut world, "E", "555");
    given_modem_with_number(&mut world, "F", "666");

    // Build Conference 1 (A + B + C)
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    when_at_command_sent(&mut world, "A", "ATD333;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");

    // Build Conference 2 (D + E + F) concurrently
    when_at_command_sent(&mut world, "D", "ATD555;");
    then_response_is(&mut world, "D", "OK");
    then_wait_for_response_containing(&mut world, "E", "RING");
    when_at_command_sent(&mut world, "E", "ATA");
    then_wait_for_response_containing(&mut world, "D", "RING");

    when_at_command_sent(&mut world, "D", "ATD666;");
    then_response_is(&mut world, "D", "OK");
    then_wait_for_response_containing(&mut world, "E", "RING");
    then_wait_for_response_containing(&mut world, "F", "RING");
    when_at_command_sent(&mut world, "F", "ATA");
    then_wait_for_response_containing(&mut world, "D", "RING");

    when_at_command_sent(&mut world, "D", "AT+CHLD=3");
    then_response_is(&mut world, "D", "OK");
    then_wait_for_response_containing(&mut world, "E", "RING");

    // Verify Group 1: A has active conference with B and C
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,1,222");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,0,0,1,333");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // Verify Group 2: D has active conference with E and F
    when_at_command_sent(&mut world, "D", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "D", "+CLCC: 1,0,0,0,1,555");
    then_wait_for_response_containing(&mut world, "D", "+CLCC: 2,0,0,0,1,666");
    then_wait_for_response_containing(&mut world, "D", "OK");

    // B hangs up from Group 1
    when_at_command_sent(&mut world, "B", "ATH");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Verify Group 1: A's remaining call with C degrades to mpty=0
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,0,0,0,333");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // Verify Group 2 is completely isolated and unaffected (still mpty=1 on both
    // calls)
    when_at_command_sent(&mut world, "D", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "D", "+CLCC: 1,0,0,0,1,555");
    then_wait_for_response_containing(&mut world, "D", "+CLCC: 2,0,0,0,1,666");
    then_wait_for_response_containing(&mut world, "D", "OK");
}

#[test]
fn test_no_duplicate_ring_to_already_held_peer_on_second_hold_dial() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");
    given_modem_with_number(&mut world, "D", "444");

    // 1. A calls B, B answers
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // 2. A calls C, C answers (puts B on hold)
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING"); // B receives HOLD URC
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // At this point: B is Held (has consumed its 1 hold RING), C is Active.
    // 3. A calls D (puts C on hold, B was ALREADY on hold)
    when_at_command_sent(&mut world, "A", "ATD444;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "C", "RING"); // C receives HOLD URC
    then_wait_for_response_containing(&mut world, "D", "RING");

    // Now query B: B should NOT have any buffered/unconsumed duplicate RING!
    when_at_command_sent(&mut world, "B", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "B", "+CLCC: 1,1,1,0,0,333,129");
    then_wait_for_response_containing(&mut world, "B", "OK");
}

#[test]
fn test_active_conference_with_waiting_call_hold_and_accept() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");
    given_modem_with_number(&mut world, "D", "444");

    // 1. Establish 3-way conference between A, B, and C
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");

    // 2. Incoming call from D while conference is active -> Waiting
    when_at_command_sent(&mut world, "D", "ATD333;");
    then_response_is(&mut world, "D", "OK");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // Verify A has 2 active conference calls and 1 waiting call
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,1,111");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,0,0,1,222");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 3,1,5,0,0,444");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // 3. AT+CHLD=2: Hold active conference, answer waiting call (D)
    when_at_command_sent(&mut world, "A", "AT+CHLD=2");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    then_wait_for_response_containing(&mut world, "C", "RING");
    then_wait_for_response_containing(&mut world, "D", "RING");

    // Verify: Calls 1 & 2 are Held (mpty=1), Call 3 is Active
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,1,0,1,111");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,1,0,1,222");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 3,1,0,0,0,444");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // 4. AT+CHLD=3: Merge D into the conference group -> All 3 active with mpty=1
    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    then_wait_for_response_containing(&mut world, "C", "RING");

    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,1,111");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,0,0,1,222");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 3,1,0,0,1,444");
    then_wait_for_response_containing(&mut world, "A", "OK");
}

#[test]
fn test_active_conference_with_waiting_call_reject_udub() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");
    given_modem_with_number(&mut world, "D", "444");

    // 1. Establish 3-way conference between A, B, and C
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");

    // 2. Incoming call from D while conference is active -> Waiting
    when_at_command_sent(&mut world, "D", "ATD333;");
    then_response_is(&mut world, "D", "OK");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // 3. AT+CHLD=0: Reject waiting call (UDUB), conference remains intact
    when_at_command_sent(&mut world, "A", "AT+CHLD=0");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "D", "RING");

    // Verify: Calls 1 & 2 are still Active with mpty=1, Call 3 is gone
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,1,111");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 2,0,0,0,1,222");
    then_wait_for_response_containing(&mut world, "A", "OK");
}

#[test]
fn test_active_conference_with_waiting_call_release_and_accept() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "333");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");
    given_modem_with_number(&mut world, "D", "444");

    // 1. Establish 3-way conference between A, B, and C
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    then_wait_for_response_containing(&mut world, "C", "RING");
    when_at_command_sent(&mut world, "C", "ATA");
    then_wait_for_response_containing(&mut world, "A", "RING");

    when_at_command_sent(&mut world, "A", "AT+CHLD=3");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");

    // 2. Incoming call from D while conference is active -> Waiting
    when_at_command_sent(&mut world, "D", "ATD333;");
    then_response_is(&mut world, "D", "OK");
    then_wait_for_response_containing(&mut world, "A", "RING");

    // 3. AT+CHLD=1: Release all active conference calls (B and C), answer waiting
    //    call (D)
    when_at_command_sent(&mut world, "A", "AT+CHLD=1");
    then_response_is(&mut world, "A", "OK");
    then_wait_for_response_containing(&mut world, "B", "RING");
    then_wait_for_response_containing(&mut world, "C", "RING");
    then_wait_for_response_containing(&mut world, "D", "RING");

    // Verify: Only Call 3 (D) is Active with mpty=0, Calls 1 & 2 are gone
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 3,1,0,0,0,444");
    then_wait_for_response_containing(&mut world, "A", "OK");
}
