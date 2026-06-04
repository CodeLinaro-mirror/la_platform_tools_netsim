// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{steps::*, world::World};

// Scenario: Query Operator Selection
//   Given a modem "A"
//   When AT command "AT+COPS?" is sent to "A"
//   Then response from "A" is "+COPS: 0,2,310260"
//   When AT command "AT+COPS=3,0" is sent to "A"
//   And AT command "AT+COPS?" is sent to "A"
//   Then response from "A" is '+COPS: 0,0,"Android Virtual Operator"'
#[test]
fn test_cops_query() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // 1. Default should be format 0 (long alphanumeric)
    when_at_command_sent(&mut world, "A", "AT+COPS?");
    then_response_is(&mut world, "A", "+COPS: 0,0,\"Android Virtual Operator\"");
    then_response_is(&mut world, "A", "OK");

    // 2. Set format to 1 (short alphanumeric)
    when_at_command_sent(&mut world, "A", "AT+COPS=3,1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+COPS?");
    then_response_is(&mut world, "A", "+COPS: 0,1,\"Android\"");
    then_response_is(&mut world, "A", "OK");

    // 3. Set format to 2 (numeric)
    when_at_command_sent(&mut world, "A", "AT+COPS=3,2");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+COPS?");
    then_response_is(&mut world, "A", "+COPS: 0,2,310260");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Query Signal Quality
//   Given a modem "A"
//   When AT command "AT+CSQ" is sent to "A"
//   Then response from "A" is "+CSQ: 20,99"
//   And response from "A" is "OK"
#[test]
fn test_csq_query() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CSQ");

    then_response_is(&mut world, "A", "+CSQ: 20,99");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Network Registration on Radio ON
//   Given a modem "A"
//   When AT command "AT+CFUN=0" is sent to "A"
//   And AT command "AT+CREG=1" is sent to "A"
//   And AT command "AT+CGREG=1" is sent to "A"
//   And AT command "AT+CEREG=1" is sent to "A"
//   And AT command "AT+CFUN=1" is sent to "A"
//   Then response from "A" is "+CREG: 1"
//   And response from "A" is "+CGREG: 1"
//   And response from "A" is "+CEREG: 1"
//   And response from "A" is "OK"
#[test]
fn test_network_registration() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Turn radio OFF first to simulate clean boot sequence
    when_at_command_sent(&mut world, "A", "AT+CFUN=0");
    then_response_is(&mut world, "A", "OK");

    // Enable unsolicited reports
    when_at_command_sent(&mut world, "A", "AT+CREG=1");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CGREG=1");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CEREG=1");
    then_response_is(&mut world, "A", "OK");

    // Turn radio ON which returns only OK synchronously
    when_at_command_sent(&mut world, "A", "AT+CFUN=1");
    then_response_is(&mut world, "A", "OK");

    // Advance time by 10ms to trigger the AttachNetwork event and send URCs!
    when_time_advances_ms(&mut world, 10);

    // Verify unsolicited reports arrive in correct order
    then_response_is(&mut world, "A", "+CREG: 1");
    then_response_is(&mut world, "A", "+CGREG: 1");
    then_response_is(&mut world, "A", "+CEREG: 1");
    then_response_is(&mut world, "A", "+CSQ: 20,99");
}

// Scenario: Set dynamic registration status
//   Given a modem "A"
//   When voice registration is set to Roaming (5)
//   Then unsolicited response from "A" is "+CREG: 5"
//   When data registration is set to Denied (3)
//   Then unsolicited response from "A" is "+CGREG: 3"
#[test]
fn test_set_registration_status() {
    use modem_rs::RegistrationStatus;
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Enable unsolicited reports first to make it spec-compliant
    when_at_command_sent(&mut world, "A", "AT+CREG=1");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CGREG=1");
    then_response_is(&mut world, "A", "OK");

    let id_a = world.modems.get("A").unwrap().0;

    // Set Voice
    when_voice_registration_set(&mut world, id_a, RegistrationStatus::Roaming);
    then_response_is(&mut world, "A", "+CREG: 5");

    // Set Data
    when_data_registration_set(&mut world, id_a, RegistrationStatus::Denied);
    then_response_is(&mut world, "A", "+CGREG: 3");
}

// Scenario: Query Extended Signal Quality
//   Given a modem "A"
//   When AT command "AT+CESQ" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_query_extended_signal_quality() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CESQ");

    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set dynamic signal strength
//   Given a modem "A"
//   When signal strength is set to 25, 0
//   Then response from "A" to "AT+CSQ" is "+CSQ: 25,0"
#[test]
fn test_set_signal_strength() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Check default
    when_at_command_sent(&mut world, "A", "AT+CSQ");
    then_response_is(&mut world, "A", "+CSQ: 20,99");
    then_response_is(&mut world, "A", "OK");

    // Change value
    let id_a = world.modems.get("A").unwrap().0;
    when_signal_strength_set(&mut world, id_a, 25, 0);

    // Check new value
    when_at_command_sent(&mut world, "A", "AT+CSQ");
    then_response_is(&mut world, "A", "+CSQ: 25,0");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Network Registration after Radio Cycle (ON -> OFF -> ON)
//   Given a modem "A"
//   When AT command "AT+CREG=1" is sent to "A"
//   And AT command "AT+CFUN=1" is sent to "A"
//   And time advances 10 ms
//   Then response from "A" is "+CREG: 1"
//   And response from "A" is "+CSQ: 20,99"
//   When AT command "AT+CFUN=0" is sent to "A"
//   Then response from "A" is "+CREG: 0"
//   And response from "A" is "OK"
//   When AT command "AT+CFUN=1" is sent to "A"
//   And time advances 10 ms
//   Then response from "A" is "+CREG: 1"
//   And response from "A" is "+CSQ: 20,99"
#[test]
fn test_network_registration_radio_cycle() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // 1. Turn radio ON and check reactive URCs
    when_at_command_sent(&mut world, "A", "AT+CREG=1");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CFUN=1");
    then_response_is(&mut world, "A", "OK");

    when_time_advances_ms(&mut world, 10);
    then_response_is(&mut world, "A", "+CREG: 1");
    then_response_is(&mut world, "A", "+CSQ: 20,99");

    // 2. Turn radio OFF (drops registration and sends URC synchronously)
    when_at_command_sent(&mut world, "A", "AT+CFUN=0");
    then_response_is(&mut world, "A", "+CREG: 0");
    then_response_is(&mut world, "A", "OK");

    // 3. Turn radio ON again and verify reactive URC triggers again!
    when_at_command_sent(&mut world, "A", "AT+CFUN=1");
    then_response_is(&mut world, "A", "OK");

    when_time_advances_ms(&mut world, 10);
    then_response_is(&mut world, "A", "+CREG: 1");
    then_response_is(&mut world, "A", "+CSQ: 20,99");
}
