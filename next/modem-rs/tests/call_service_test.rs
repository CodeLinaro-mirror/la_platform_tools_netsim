// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{common::constants::*, steps::*, world::World};

// Scenario: Emergency Call
//   Given a modem "A"
//   When AT command "ATD911;" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_emergency_call() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", &format!("ATD{TEST_EMERGENCY_NUMBER};"));
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Standard Call
//   Given a modem "A"
//   When AT command "ATD1234567;" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_standard_call() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Dial international number
    when_at_command_sent(&mut world, "A", &format!("ATD+{TEST_PHONE_NUMBER_ALT};"));
    then_response_is(&mut world, "A", "OK");

    // Verify CLCC shows Dialing state (2) and International ToA (145)
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_response_contains(
        &mut world,
        "A",
        &format!("+CLCC: 1,0,2,0,0,\"+{TEST_PHONE_NUMBER_ALT}\",{TOA_INTERNATIONAL}"),
    );
    then_response_contains(&mut world, "A", "OK");
    then_no_response(&mut world, "A");
}

// Scenario: Receive Ring
//   Given a modem "A"
//   When AT command "RING" is sent to "A"
//   Then response from "A" is "RING"
//   When AT command "AT+CLCC" is sent to "A"
//   Then response from "A" contains "+CLCC: 1,1,4,0,0,\"\",129"
//   And response from "A" contains "OK"
#[test]
fn test_ring() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "RING");
    then_response_is(&mut world, "A", "RING");

    // Verify call state (Incoming)
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_response_contains(&mut world, "A", &format!("+CLCC: 1,1,4,0,0,\"\",{TOA_NATIONAL}"));
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Query Current Calls
//   Given a modem "A"
//   And a modem "B" with number "111"
//   And a modem "C" with number "222"
//   When AT command "ATD111;" is sent to "A"
//   Then response from "A" is "OK"
//   When AT command "ATA" is sent to "B"
//   Then response from "B" is "OK"
//   When AT command "ATD222;" is sent to "A"
//   Then response from "A" is "OK"
//   When AT command "ATA" is sent to "C"
//   Then response from "C" is "OK"
//   When AT command "AT+CLCC" is sent to "A"
//   Then response from "A" contains "+CLCC: 1,0,1,0,0,\"111\",129"
//   And response from "A" contains "+CLCC: 2,0,0,0,0,\"222\",129"
//   And response from "A" contains "OK"
#[test]
fn test_query_current_calls() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    given_modem_with_number(&mut world, "B", CALL_PEER_B);
    given_modem_with_number(&mut world, "C", CALL_PEER_C);

    // A calls B, B answers
    when_at_command_sent(&mut world, "A", &format!("ATD{CALL_PEER_B};"));
    then_response_is(&mut world, "A", "OK");
    then_response_is(&mut world, "B", "RING");

    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_response_is(&mut world, "A", "OK"); // Connection established

    // A calls C, B goes on hold, C answers
    when_at_command_sent(&mut world, "A", &format!("ATD{CALL_PEER_C};"));
    then_response_is(&mut world, "A", "OK");
    then_response_is(&mut world, "C", "RING");

    when_at_command_sent(&mut world, "C", "ATA");
    then_response_is(&mut world, "C", "OK");
    then_response_is(&mut world, "A", "OK"); // Connection established

    // Query A's calls
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    // Order might vary, so check contains
    then_response_contains(
        &mut world,
        "A",
        &format!("+CLCC: 1,0,1,0,0,\"{CALL_PEER_B}\",{TOA_NATIONAL}"),
    );
    then_response_contains(
        &mut world,
        "A",
        &format!("+CLCC: 2,0,0,0,0,\"{CALL_PEER_C}\",{TOA_NATIONAL}"),
    );
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Call Ring Timeout
//   Given a modem "A"
//   And a modem "B" with number "111"
//   And a modem "C" with number "222"
//   When AT command "ATD111;" is sent to "A"
//   And AT command "ATA" is sent to "B"
//   And AT command "ATD222;" is sent to "A"
//   Then check "C" is ringing (Incoming)
//   When time advances 30100 ms
//   Then check "C" is idle
#[test]
fn test_call_ring_timeout() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", TEST_PHONE_NUMBER_LONG_A);
    given_modem_with_number(&mut world, "B", CALL_PEER_B);
    given_modem_with_number(&mut world, "C", CALL_PEER_C);

    // A calls B, B answers
    when_at_command_sent(&mut world, "A", &format!("ATD{CALL_PEER_B};"));
    then_response_is(&mut world, "A", "OK");
    then_response_is(&mut world, "B", "RING");

    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_response_is(&mut world, "A", "OK"); // Connected

    // A calls C
    when_at_command_sent(&mut world, "A", &format!("ATD{CALL_PEER_C};"));
    then_response_is(&mut world, "A", "OK");

    // Verify C has Incoming call - First consume RING
    then_response_is(&mut world, "C", "RING");

    // Check AT+CLCC on C
    when_at_command_sent(&mut world, "C", "AT+CLCC");
    then_response_contains(
        &mut world,
        "C",
        &format!("+CLCC: 1,1,4,0,0,\"+{}\",{}", TEST_PHONE_NUMBER_LONG_A, TOA_INTERNATIONAL),
    );
    then_response_is(&mut world, "C", "OK");

    // Advance time > 30s
    when_time_advances_ms(&mut world, 30100);

    // Verify C is idle (AT+CLCC returns just OK)
    when_at_command_sent(&mut world, "C", "AT+CLCC");
    then_response_is(&mut world, "C", "OK");
}

// Scenario: Set Mute
//   Given a modem "A"
//   When AT command "AT+CMUT=1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_set_mute() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CMUT=1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Query Mute
//   Given a modem "A"
//   When AT command "AT+CMUT=1" is sent to "A"
//   Then response from "A" is "OK"
//   When AT command "AT+CMUT?" is sent to "A"
//   Then response from "A" is "+CMUT: 1"
//   And response from "A" is "OK"
#[test]
fn test_query_mute() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CMUT=1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CMUT?");
    then_response_is(&mut world, "A", "+CMUT: 1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Send DTMF
//   Given a modem "A"
//   When AT command "AT+VTS=1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_send_dtmf() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+VTS=1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set Emergency Mode
//   Given a modem "A"
//   When AT command "AT+WSOS=1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_set_emergency_mode() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+WSOS=1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Query Emergency Mode
//   Given a modem "A"
//   When AT command "AT+WSOS=1" is sent to "A"
//   Then response from "A" is "OK"
//   When AT command "AT+WSOS?" is sent to "A"
//   Then response from "A" is "+WSOS: 1"
//   And response from "A" is "OK"
#[test]
fn test_query_emergency_mode() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+WSOS=1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+WSOS?");
    then_response_is(&mut world, "A", "+WSOS: 1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: External Incoming Call (Console)
//   Given a modem "A"
//   When external call from "123456" matches "A"
//   Then response from "A" is "RING"
//   And response from "A" contains "+CLIP: \"123456\",129,,,,0"
//   When time, advances > 1s (call ring timeout)
//   Then check "A" is idle
#[test]
fn test_external_incoming_call() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Enable CLIP
    when_at_command_sent(&mut world, "A", "AT+CLIP=1");
    then_response_is(&mut world, "A", "OK");

    // Inject call
    when_incoming_call_received(&mut world, "A", TEST_PHONE_NUMBER);

    // Expect RING
    then_response_is(&mut world, "A", "RING");
    // Expect +CLIP
    then_response_contains(
        &mut world,
        "A",
        &format!("+CLIP: \"{TEST_PHONE_NUMBER}\",{}", TOA_NATIONAL),
    );

    // Verify call state (Incoming)
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    // ID=1, Direction=1(Incoming), State=4(Incoming), Voice=0(Voice), Multiparty=0,
    // Number="123456", Type=129
    then_response_contains(
        &mut world,
        "A",
        &format!("+CLCC: 1,1,4,0,0,\"{TEST_PHONE_NUMBER}\",{}", TOA_NATIONAL),
    );
    then_response_is(&mut world, "A", "OK");

    // Wait for timeout
    when_time_advances_ms(&mut world, 2000);

    // Call should be gone
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: External Call Control (Answer/Hangup)
//   Given a modem "A"
//   When AT command "ATD123;" is sent to "A"
//   Then response from "A" is "OK"
//   When external answer triggered for "A"
//   Then response from "A" is "OK" (representing CONNECT)
//   And check call state is Active
//   When external hangup triggered for "A"
//   Then response from "A" is "NO CARRIER"
//   And check call state is Idle
#[test]
fn test_external_call_control() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Dial
    when_at_command_sent(&mut world, "A", &format!("ATD{CALL_PEER_EXT};"));
    then_response_is(&mut world, "A", "OK");

    // Remote Answer
    when_external_call_answered(&mut world, "A");
    then_response_is(&mut world, "A", "OK");

    // Verify Active
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    // ID=1, Dir=0(Out), State=0(Active), ...
    then_response_contains(
        &mut world,
        "A",
        &format!("+CLCC: 1,0,0,0,0,\"{CALL_PEER_EXT}\",{}", TOA_NATIONAL),
    );
    then_response_is(&mut world, "A", "OK");

    // Remote Hangup
    when_external_call_hungup(&mut world, "A");
    then_response_is(&mut world, "A", "NO CARRIER");

    // Verify Idle
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: External Call Hold
//   Given a modem "A"
//   When AT command "ATD123;" is sent to "A"
//   Then response from "A" is "OK"
//   When external answer triggered
//   Then response from "A" is "OK"
//   When external hold (on) triggered
//   Then AT+CLCC indicates Held (State 1)
//   When external hold (off) triggered
//   Then AT+CLCC indicates Active (State 0)
#[test]
fn test_external_call_hold() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Dial and Answer to get Active call
    when_at_command_sent(&mut world, "A", &format!("ATD{CALL_PEER_EXT};"));
    then_response_is(&mut world, "A", "OK");
    when_external_call_answered(&mut world, "A");
    then_response_is(&mut world, "A", "OK");

    // Hold ON
    when_external_call_held(&mut world, "A", true);

    // Check State (Held = 1)
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    // +CLCC: 1,0,1,... (State 1)
    then_response_contains(
        &mut world,
        "A",
        &format!("+CLCC: 1,0,1,0,0,\"{CALL_PEER_EXT}\",{}", TOA_NATIONAL),
    );
    then_response_is(&mut world, "A", "OK");

    // Hold OFF (Resume)
    when_external_call_held(&mut world, "A", false);

    // Check State (Active = 0)
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    // +CLCC: 1,0,0,... (State 0)
    then_response_contains(
        &mut world,
        "A",
        &format!("+CLCC: 1,0,0,0,0,\"{CALL_PEER_EXT}\",{}", TOA_NATIONAL),
    );
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_dial_speed_dial_does_not_fallback() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Dial *999# which starts with *99 and ends with #, but is not standard GPRS
    // dial format. It should NOT fall back, but remain handled by CallService
    // as a normal voice dial attempt (returns OK). If it had fallen back, it
    // would have returned CONNECT!
    when_at_command_sent(&mut world, "A", "ATD*999#");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_emergency_dial_syntax() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Emergency with category and CLIR
    when_at_command_sent(&mut world, "A", &format!("ATD{}@1,#I;", TEST_EMERGENCY_NUMBER));
    then_response_is(&mut world, "A", "OK");

    // Verify it is NOT in active calls (InitiateEmergencyCall is no-op)
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_response_is(&mut world, "A", "OK");

    // Normal call to non-existent peer should be in Dialing state
    when_at_command_sent(&mut world, "A", &format!("ATD{CALL_PEER_ALT_2};"));
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_response_contains(
        &mut world,
        "A",
        &format!("+CLCC: 1,0,2,0,0,\"{CALL_PEER_ALT_2}\",{}", TOA_NATIONAL),
    );
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_dial_clir_semicolon() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Dial with CLIR 'i' and semicolon
    when_at_command_sent(&mut world, "A", &format!("ATD{CALL_PEER_ALT_2}i;"));
    then_response_is(&mut world, "A", "OK");

    // Verify it dialed "12345" (clean number)
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_response_contains(
        &mut world,
        "A",
        &format!("+CLCC: 1,0,2,0,0,\"{CALL_PEER_ALT_2}\",{}", TOA_NATIONAL),
    );
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_invalid_dial_syntax() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Plus in the middle should be rejected
    when_at_command_sent(&mut world, "A", "ATD123+456;");
    then_response_is(&mut world, "A", "ERROR");

    // Multiple pluses should be rejected
    when_at_command_sent(&mut world, "A", "ATD++123;");
    then_response_is(&mut world, "A", "ERROR");

    // Plus at the beginning should be accepted
    when_at_command_sent(&mut world, "A", &format!("ATD+{CALL_PEER_ALT_2};"));
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_dtmf_validation() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Valid DTMFs
    when_at_command_sent(&mut world, "A", "AT+VTS=1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+VTS=*");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+VTS=A,10");
    then_response_is(&mut world, "A", "OK");

    // Invalid DTMFs
    when_at_command_sent(&mut world, "A", "AT+VTS=X");
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(&mut world, "A", "AT+VTS=12");
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(&mut world, "A", "AT+VTS=1,A");
    then_response_is(&mut world, "A", "ERROR");
}

#[test]
fn test_standard_call_with_leading_plus_routing() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", TEST_PHONE_NUMBER_LONG_A);
    given_modem_with_number(&mut world, "B", TEST_PHONE_NUMBER_LONG_B);

    // A dials B's number with leading '+'
    when_at_command_sent(&mut world, "A", &format!("ATD+{TEST_PHONE_NUMBER_LONG_B};"));
    then_response_is(&mut world, "A", "OK");

    // Verify B receives the incoming RING
    then_response_is(&mut world, "B", "RING");
}

#[test]
fn test_remote_call_initiation() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", TEST_PHONE_NUMBER_LONG_A);
    given_modem_with_number(&mut world, "B", TEST_PHONE_NUMBER_LONG_B);

    // A calls B via remote call
    when_at_command_sent(&mut world, "A", &format!("AT+REMOTECALL={TEST_PHONE_NUMBER_LONG_B}"));
    then_response_is(&mut world, "A", "OK");

    // B should receive RING
    then_response_is(&mut world, "B", "RING");
}

#[test]
fn test_clir_dial_suffixes() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", TEST_PHONE_NUMBER_LONG_A);
    given_modem_with_number(&mut world, "B", TEST_PHONE_NUMBER_LONG_B);

    // Enable CLIP on B
    when_at_command_sent(&mut world, "B", "AT+CLIP=1");
    then_response_is(&mut world, "B", "OK");

    // Scenario 1: Dial with CLIR suppression 'i' (allow presentation)
    when_at_command_sent(&mut world, "A", &format!("ATD{TEST_PHONE_NUMBER_LONG_B}i;"));
    then_response_is(&mut world, "A", "OK");

    // B should receive RING and +CLIP presenting A's number
    then_response_contains(&mut world, "B", "RING");
    then_response_contains(
        &mut world,
        "B",
        &format!("+CLIP: \"+{TEST_PHONE_NUMBER_LONG_A}\",145,,,,0"),
    );

    // Hang up
    when_at_command_sent(&mut world, "A", "ATH");
    then_response_is(&mut world, "A", "OK");

    // Scenario 2: Dial with CLIR invocation 'I' (restrict presentation)
    when_at_command_sent(&mut world, "A", &format!("ATD{TEST_PHONE_NUMBER_LONG_B}I;"));
    then_response_is(&mut world, "A", "OK");

    // B should receive RING and +CLIP with restricted caller ID
    then_response_contains(&mut world, "B", "RING");
    then_response_contains(&mut world, "B", "+CLIP: \"\",129,,,,1");
}

#[test]
fn test_fdn_dial_restriction() {
    let mut world = World::new();
    given_modem_with_fdn_sim_profile(&mut world, "A");
    given_modem_with_number(&mut world, "B", TEST_PHONE_NUMBER_LONG_B);

    // Enable verbose CME errors
    when_at_command_sent(&mut world, "A", "AT+CMEE=1");
    then_response_is(&mut world, "A", "OK");

    // By default FDN is disabled, so dialing B's number should succeed
    when_at_command_sent(&mut world, "A", &format!("ATD{TEST_PHONE_NUMBER_LONG_B};"));
    then_response_is(&mut world, "A", "OK");
    then_response_is(&mut world, "B", "RING");

    // Hang up B
    when_at_command_sent(&mut world, "B", "ATH");
    then_response_is(&mut world, "B", "OK");
    then_response_is(&mut world, "A", "");
    then_response_is(&mut world, "A", "NO CARRIER");

    // Enable FDN lock using PIN2 "5678"
    when_at_command_sent(&mut world, "A", "AT+CLCK=\"FD\",1,\"5678\"");
    then_response_is(&mut world, "A", "OK");

    // Query FDN status -> should be 1 (enabled)
    when_at_command_sent(&mut world, "A", "AT+CLCK=\"FD\",2");
    then_response_contains(&mut world, "A", "+CLCK: 1");
    then_response_is(&mut world, "A", "OK");

    // Try to dial B's number (not in FDN list) -> should fail with CME ERROR 56
    when_at_command_sent(&mut world, "A", &format!("ATD{TEST_PHONE_NUMBER_LONG_B};"));
    then_response_is(&mut world, "A", "+CME ERROR: 56");

    // Dial number in FDN list (12345) -> should succeed
    when_at_command_sent(&mut world, "A", "ATD12345;");
    then_response_is(&mut world, "A", "OK");

    // Hang up A
    when_at_command_sent(&mut world, "A", "ATH");
    then_response_is(&mut world, "A", "OK");

    // Dial number starting with FDN list entry (1234567) -> should succeed (prefix
    // match)
    when_at_command_sent(&mut world, "A", "ATD1234567;");
    then_response_is(&mut world, "A", "OK");

    // Hang up A
    when_at_command_sent(&mut world, "A", "ATH");
    then_response_is(&mut world, "A", "OK");

    // Dial number that is prefix of FDN entry but shorter (1234) -> should fail
    when_at_command_sent(&mut world, "A", "ATD1234;");
    then_response_is(&mut world, "A", "+CME ERROR: 56");

    // Disable FDN lock
    when_at_command_sent(&mut world, "A", "AT+CLCK=\"FD\",0,\"5678\"");
    then_response_is(&mut world, "A", "OK");

    // Dial B's number again -> should succeed now
    when_at_command_sent(&mut world, "A", &format!("ATD{TEST_PHONE_NUMBER_LONG_B};"));
    then_response_is(&mut world, "A", "OK");
    then_response_is(&mut world, "B", "RING");
}

#[test]
fn test_fdn_emergency_call_bypass() {
    let mut world = World::new();
    given_modem_with_fdn_sim_profile(&mut world, "A");

    // Enable FDN lock using PIN2 "5678"
    when_at_command_sent(&mut world, "A", "AT+CLCK=\"FD\",1,\"5678\"");
    then_response_is(&mut world, "A", "OK");

    // Dial emergency call (911) -> should bypass FDN and return OK
    when_at_command_sent(&mut world, "A", "ATD911;");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_fdn_pin2_toggle_failure() {
    let mut world = World::new();
    given_modem_with_fdn_sim_profile(&mut world, "A");

    // Try to enable FDN lock with INCORRECT PIN2 "0000" -> should fail with ERROR
    when_at_command_sent(&mut world, "A", "AT+CLCK=\"FD\",1,\"0000\"");
    then_response_is(&mut world, "A", "ERROR");

    // Verify FDN lock status remains 0 (disabled)
    when_at_command_sent(&mut world, "A", "AT+CLCK=\"FD\",2");
    then_response_contains(&mut world, "A", "+CLCK: 0");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_clir_clcc_number_hiding() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", TEST_PHONE_NUMBER_LONG_A);
    given_modem_with_number(&mut world, "B", TEST_PHONE_NUMBER_LONG_B);

    // Enable CLIP on B
    when_at_command_sent(&mut world, "B", "AT+CLIP=1");
    then_response_is(&mut world, "B", "OK");

    // Dial B from A with CLIR invocation 'I' (restrict presentation)
    when_at_command_sent(&mut world, "A", &format!("ATD{TEST_PHONE_NUMBER_LONG_B}I;"));
    then_response_is(&mut world, "A", "OK");

    // B should receive RING and +CLIP with restricted caller ID
    then_response_contains(&mut world, "B", "RING");
    then_response_contains(&mut world, "B", "+CLIP: \"\",129,,,,1");

    // B queries current calls list -> should show incoming call with hidden caller
    // number
    when_at_command_sent(&mut world, "B", "AT+CLCC");
    then_response_contains(&mut world, "B", "+CLCC: 1,1,4,0,0,\"\",129");
    then_response_is(&mut world, "B", "OK");
}

#[test]
fn test_fdn_international_matching() {
    let mut world = World::new();
    given_modem_with_fdn_sim_profile(&mut world, "A");

    // Enable FDN lock using PIN2 "5678"
    when_at_command_sent(&mut world, "A", "AT+CLCK=\"FD\",1,\"5678\"");
    then_response_is(&mut world, "A", "OK");

    // Dial international FDN number with '+' prefix -> should succeed
    when_at_command_sent(&mut world, "A", "ATD+16505550100;");
    then_response_is(&mut world, "A", "OK");

    // Hang up A
    when_at_command_sent(&mut world, "A", "ATH");
    then_response_is(&mut world, "A", "OK");

    // Dial same international number WITHOUT '+' prefix -> should also succeed due
    // to normalization
    when_at_command_sent(&mut world, "A", "ATD16505550100;");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_dial_with_pause_modifier() {
    let mut world = World::new();
    given_modem_with_number(&mut world, "A", "987654");
    given_modem_with_number(&mut world, "B", "123456");

    // Enable CLIP on B to receive caller ID
    when_at_command_sent(&mut world, "B", "AT+CLIP=1");
    then_response_is(&mut world, "B", "OK");

    // Dial B's number (123456) from A with a pause modifier and DTMF suffix
    when_at_command_sent(&mut world, "A", "ATD123456,1234;");
    then_response_is(&mut world, "A", "OK");

    // Verify B receives the incoming RING and +CLIP with A's number
    then_response_contains(&mut world, "B", "RING");
    then_response_contains(&mut world, "B", "+CLIP: \"987654\",129,,,,0");

    // A queries current calls list -> should show dialing/active call with B's
    // clean number
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_response_contains(&mut world, "A", "+CLCC: 1,0,2,0,0,\"123456\",129");
    then_response_is(&mut world, "A", "OK");
}
