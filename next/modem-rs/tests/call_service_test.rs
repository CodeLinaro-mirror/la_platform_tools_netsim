use crate::{steps::*, world::World};

// Scenario: Emergency Call
//   Given a modem "A"
//   When AT command "ATD911;" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_emergency_call() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "ATD911;");
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
    when_at_command_sent(&mut world, "A", "ATD1234567;");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Receive Ring
//   Given a modem "A"
//   When AT command "RING" is sent to "A"
//   Then response from "A" is "RING"
//   When AT command "AT+CLCC" is sent to "A"
//   Then response from "A" contains "+CLCC: 1,1,3,0,0,\"\",129"
//   And response from "A" contains "OK"
#[test]
fn test_ring() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "RING");
    then_response_is(&mut world, "A", "RING");

    // Verify call state (Incoming/Alerting)
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_response_contains(&mut world, "A", "+CLCC: 1,1,3,0,0,\"\",129");
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
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    // A calls B, B answers
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_response_is(&mut world, "B", "RING");

    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_response_is(&mut world, "A", "OK"); // Connection established

    // A calls C, B goes on hold, C answers
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");
    then_response_is(&mut world, "C", "RING");

    when_at_command_sent(&mut world, "C", "ATA");
    then_response_is(&mut world, "C", "OK");
    then_response_is(&mut world, "A", "OK"); // Connection established

    // Query A's calls
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    // Order might vary, so check contains
    then_response_contains(&mut world, "A", "+CLCC: 1,0,1,0,0,\"111\",129");
    then_response_contains(&mut world, "A", "+CLCC: 2,0,0,0,0,\"222\",129");
}

// Scenario: Call Ring Timeout
//   Given a modem "A"
//   And a modem "B" with number "111"
//   And a modem "C" with number "222"
//   When AT command "ATD111;" is sent to "A"
//   And AT command "ATA" is sent to "B"
//   And AT command "ATD222;" is sent to "A"
//   Then check "C" is ringing (Alerting)
//   When time advances 30100 ms
//   Then check "C" is idle
#[test]
fn test_call_ring_timeout() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    given_modem_with_number(&mut world, "B", "111");
    given_modem_with_number(&mut world, "C", "222");

    // A calls B, B answers
    when_at_command_sent(&mut world, "A", "ATD111;");
    then_response_is(&mut world, "A", "OK");
    then_response_is(&mut world, "B", "RING");

    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_response_is(&mut world, "A", "OK"); // Connected

    // A calls C
    when_at_command_sent(&mut world, "A", "ATD222;");
    then_response_is(&mut world, "A", "OK");

    // Verify C is alerting (Incoming call) - First consume RING
    then_response_is(&mut world, "C", "RING");

    // Check AT+CLCC on C
    when_at_command_sent(&mut world, "C", "AT+CLCC");
    then_response_contains(&mut world, "C", "+CLCC: 1,1,3,0,0,\"\",129");

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
