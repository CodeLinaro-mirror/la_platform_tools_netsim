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

    // Enable radio and attach
    when_at_command_sent(&mut world, "A", "AT+CFUN=1");
    then_response_is(&mut world, "A", "OK");
    when_time_advances_ms(&mut world, 10);
    let csq_response = "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,20,88,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647";
    then_response_is(&mut world, "A", csq_response);

    // 1. Default should be format 2 (numeric)
    when_at_command_sent(&mut world, "A", "AT+COPS?");
    then_response_is(&mut world, "A", "+COPS: 0,2,310260");
    then_response_is(&mut world, "A", "OK");

    // 2. Set format to 0 (long alphanumeric)
    when_at_command_sent(&mut world, "A", "AT+COPS=3,0");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+COPS?");
    then_response_is(&mut world, "A", "+COPS: 0,0,\"Android Virtual Operator\"");
    then_response_is(&mut world, "A", "OK");

    // 3. Set format to 1 (short alphanumeric)
    when_at_command_sent(&mut world, "A", "AT+COPS=3,1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+COPS?");
    then_response_is(&mut world, "A", "+COPS: 0,1,\"Android\"");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cops_modes() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    let csq_response = "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,20,88,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647";

    // Enable CREG URCs
    when_at_command_sent(&mut world, "A", "AT+CREG=1");
    then_response_is(&mut world, "A", "OK");

    // Initially we are NOT attached (startup)
    when_at_command_sent(&mut world, "A", "AT+CFUN=1");
    then_response_is(&mut world, "A", "OK");
    when_time_advances_ms(&mut world, 10);
    then_response_is(&mut world, "A", "+CREG: 1");
    then_response_is(&mut world, "A", csq_response);

    // 1. Test Deregister (AT+COPS=2)
    when_at_command_sent(&mut world, "A", "AT+COPS=2");
    then_response_is(&mut world, "A", "+CREG: 0");
    then_response_is(&mut world, "A", "OK");

    // Query should show mode 2
    when_at_command_sent(&mut world, "A", "AT+COPS?");
    then_response_is(&mut world, "A", "+COPS: 2,2,0");
    then_response_is(&mut world, "A", "OK");

    // 2. Test Auto Register (AT+COPS=0)
    when_at_command_sent(&mut world, "A", "AT+COPS=0");
    then_response_is(&mut world, "A", "+CREG: 1");
    then_response_is(&mut world, "A", csq_response);
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+COPS?");
    then_response_is(&mut world, "A", "+COPS: 0,2,310260");
    then_response_is(&mut world, "A", "OK");

    // 3. Test Manual Register to wrong operator (AT+COPS=1,2,\"123456\")
    when_at_command_sent(&mut world, "A", "AT+COPS=1,2,\"123456\"");
    // Registration denied (3)
    then_response_is(&mut world, "A", "+CREG: 3");
    then_response_is(&mut world, "A", "ERROR");

    // Query should show mode 0 (reverted from 1)
    when_at_command_sent(&mut world, "A", "AT+COPS?");
    then_response_is(&mut world, "A", "+COPS: 0,2,0");
    then_response_is(&mut world, "A", "OK");

    // 4. Test Manual Register to correct operator (AT+COPS=1,0,\"Android Virtual
    //    Operator\")
    when_at_command_sent(&mut world, "A", "AT+COPS=1,0,\"Android Virtual Operator\"");
    then_response_is(&mut world, "A", "+CREG: 1");
    then_response_is(&mut world, "A", csq_response);
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+COPS?");
    then_response_is(&mut world, "A", "+COPS: 1,0,\"Android Virtual Operator\"");
    then_response_is(&mut world, "A", "OK");

    // 5. Test Manual Register to wrong operator when previous was manual
    //    (AT+COPS=1,2,\"123456\")
    when_at_command_sent(&mut world, "A", "AT+COPS=1,2,\"123456\"");
    // Registration denied (3)
    then_response_is(&mut world, "A", "+CREG: 3");
    then_response_is(&mut world, "A", "ERROR");

    // Query should show mode 0 (fallback to 0, even though previous was 1)
    when_at_command_sent(&mut world, "A", "AT+COPS?");
    then_response_is(&mut world, "A", "+COPS: 0,2,0");
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

    then_response_is(
        &mut world,
        "A",
        "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,20,88,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647",
    );
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
    then_response_is(
        &mut world,
        "A",
        "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,20,88,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647",
    );
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

    // Change technology to GSM first, so GSM signal strength is active and
    // testable!
    when_at_command_sent(&mut world, "A", "AT+CTEC=1,\"1\"");
    then_response_is(&mut world, "A", "+CTEC: DONE");
    then_response_is(&mut world, "A", "OK");

    // Check default (which should now be 20,99 because we are on GSM)
    when_at_command_sent(&mut world, "A", "AT+CSQ");
    then_response_is(
        &mut world,
        "A",
        "+CSQ: 20,99,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647",
    );
    then_response_is(&mut world, "A", "OK");

    // Change value
    let id_a = world.modems.get("A").unwrap().0;
    when_signal_strength_set(&mut world, id_a, 25, 0);

    // Check new value
    when_at_command_sent(&mut world, "A", "AT+CSQ");
    then_response_is(
        &mut world,
        "A",
        "+CSQ: 25,0,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647",
    );
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
    then_response_is(
        &mut world,
        "A",
        "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,20,88,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647",
    );

    // 2. Turn radio OFF (drops registration and sends URC synchronously)
    when_at_command_sent(&mut world, "A", "AT+CFUN=0");
    then_response_is(&mut world, "A", "+CREG: 0");
    then_response_is(&mut world, "A", "OK");

    // 3. Turn radio ON again and verify reactive URC triggers again!
    when_at_command_sent(&mut world, "A", "AT+CFUN=1");
    then_response_is(&mut world, "A", "OK");

    when_time_advances_ms(&mut world, 10);
    then_response_is(&mut world, "A", "+CREG: 1");
    then_response_is(
        &mut world,
        "A",
        "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,20,88,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647",
    );
}

// Scenario: Query Operator in All Formats (Compound Query)
//   Given a modem "A"
//   When AT command "AT+COPS=3,0;+COPS?;+COPS=3,1;+COPS?;+COPS=3,2;+COPS?" is
// sent to "A"   Then response from "A" is "+COPS: 0,0,\"Android Virtual
// Operator\""   And response from "A" is "+COPS: 0,1,\"Android\""
//   And response from "A" is "+COPS: 0,2,310260"
//   And response from "A" is "OK"
#[test]
fn test_query_operator_all_formats() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Enable radio and attach
    when_at_command_sent(&mut world, "A", "AT+CFUN=1");
    then_response_is(&mut world, "A", "OK");
    when_time_advances_ms(&mut world, 10);
    let csq_response = "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,20,88,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647";
    then_response_is(&mut world, "A", csq_response);

    when_at_command_sent(&mut world, "A", "AT+COPS=3,0;+COPS?;+COPS=3,1;+COPS?;+COPS=3,2;+COPS?");
    then_response_is(&mut world, "A", "+COPS: 0,0,\"Android Virtual Operator\"");
    then_response_is(&mut world, "A", "+COPS: 0,1,\"Android\"");
    then_response_is(&mut world, "A", "+COPS: 0,2,310260");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Query Current Network Technology Mode (AT+CTEC?)
//   Given a modem "A"
//   When AT command "AT+CTEC?" is sent to "A"
//   Then response from "A" is "+CTEC: 32,40"
//   And response from "A" is "OK"
#[test]
fn test_query_current_ctec() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CTEC?");
    then_response_is(&mut world, "A", "+CTEC: 32,40");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Query Supported Network Technology Modes (AT+CTEC=?)
//   Given a modem "A"
//   When AT command "AT+CTEC=?" is sent to "A"
//   Then response from "A" is "+CTEC: 0,1,5,6"
//   And response from "A" is "OK"
#[test]
fn test_query_supported_ctec() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CTEC=?");
    then_response_is(&mut world, "A", "+CTEC: 0,1,5,6");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set Network Technology Mode (AT+CTEC=current,preferred)
//   Given a modem "A"
//   When AT command "AT+CTEC=1,"21"" is sent to "A"
//   Then response from "A" is "+CTEC: DONE"
//   And response from "A" is "OK"
#[test]
fn test_set_ctec() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CTEC=1,\"21\"");
    then_response_is(&mut world, "A", "+CTEC: DONE");
    then_response_is(&mut world, "A", "OK");

    // Verify that values are updated and queried back correctly
    when_at_command_sent(&mut world, "A", "AT+CTEC?");
    then_response_is(&mut world, "A", "+CTEC: 1,21");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_set_ctec_invalid() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Invalid current tech (99 is not supported)
    when_at_command_sent(&mut world, "A", "AT+CTEC=99,\"21\"");
    then_response_is(&mut world, "A", "ERROR");

    // Invalid preferred mask (0x200 is not supported, only 0x63 is supported)
    when_at_command_sent(&mut world, "A", "AT+CTEC=1,\"200\"");
    then_response_is(&mut world, "A", "ERROR");

    // Invalid current tech (5 is index, but we expect mask. 5 as mask is 0b101
    // which is invalid)
    when_at_command_sent(&mut world, "A", "AT+CTEC=5,\"21\"");
    then_response_is(&mut world, "A", "ERROR");
}

#[test]
fn test_set_ctec_wcdma_and_urc() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // 1. Attach to network and enable unsolicited reports with format 2 (includes
    //    AcT!)
    when_at_command_sent(&mut world, "A", "AT+CREG=2");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CGREG=2");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CEREG=2");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CFUN=1");
    then_response_is(&mut world, "A", "OK");
    when_time_advances_ms(&mut world, 10);

    // Consume the initial attachment URCs (which default to LTE act = 7!)
    then_response_is(&mut world, "A", "+CREG: 1,\"2142\",\"0000B804\",7");
    then_response_is(&mut world, "A", "+CGREG: 1,\"2142\",\"0000B804\",7");
    then_response_is(&mut world, "A", "+CEREG: 1,\"2142\",\"0000B804\",7");
    then_response_is(
        &mut world,
        "A",
        "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,20,88,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647",
    );

    // 2. Switch technology to WCDMA (current mode = 2, preferred mask = 0x21)
    when_at_command_sent(&mut world, "A", "AT+CTEC=2,\"21\"");

    // Verify CTEC response and the IMMEDIATE unsolicited URCs showing WCDMA act =
    // 2!
    then_response_is(&mut world, "A", "+CTEC: DONE");
    then_response_is(&mut world, "A", "+CREG: 1,\"2142\",\"0000B804\",2");
    then_response_is(&mut world, "A", "+CGREG: 1,\"2142\",\"0000B804\",2");
    then_response_is(&mut world, "A", "+CEREG: 1,\"2142\",\"0000B804\",2");
    then_response_is(
        &mut world,
        "A",
        "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,20,99,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647",
    );
    then_response_is(&mut world, "A", "OK");

    // Verify that tech is queried back correctly as WCDMA (current = 2!)
    when_at_command_sent(&mut world, "A", "AT+CTEC?");
    then_response_is(&mut world, "A", "+CTEC: 2,21");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cops_manual_registration_denied_all_urcs() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Enable CREG, CGREG, and CEREG URCs
    when_at_command_sent(&mut world, "A", "AT+CREG=1");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CGREG=1");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CEREG=1");
    then_response_is(&mut world, "A", "OK");

    // Enable radio
    when_at_command_sent(&mut world, "A", "AT+CFUN=1");
    then_response_is(&mut world, "A", "OK");
    when_time_advances_ms(&mut world, 10);
    // Consume attachment URCs
    then_response_is(&mut world, "A", "+CREG: 1");
    then_response_is(&mut world, "A", "+CGREG: 1");
    then_response_is(&mut world, "A", "+CEREG: 1");
    // Consume CSQ report
    let csq_response = "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,20,88,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647";
    then_response_is(&mut world, "A", csq_response);

    // Try Manual Register to wrong operator (AT+COPS=1,2,\"123456\")
    when_at_command_sent(&mut world, "A", "AT+COPS=1,2,\"123456\"");
    // We should receive Denied (3) URCs for all three: CREG, CGREG, CEREG
    then_response_is(&mut world, "A", "+CREG: 3");
    then_response_is(&mut world, "A", "+CGREG: 3");
    then_response_is(&mut world, "A", "+CEREG: 3");
    then_response_is(&mut world, "A", "ERROR");
}

#[test]
fn test_cops_mode_4() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Enable radio
    when_at_command_sent(&mut world, "A", "AT+CFUN=1");
    then_response_is(&mut world, "A", "OK");
    when_time_advances_ms(&mut world, 10);
    // Consume CSQ report
    let csq_response = "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,20,88,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647";
    then_response_is(&mut world, "A", csq_response);

    // 1. Missing oper -> ERROR
    when_at_command_sent(&mut world, "A", "AT+COPS=4");
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(&mut world, "A", "AT+COPS=4,0");
    then_response_is(&mut world, "A", "ERROR");

    // 2. Correct operator -> Success (cops_mode = 4)
    when_at_command_sent(&mut world, "A", "AT+COPS=4,0,\"Android Virtual Operator\"");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+COPS?");
    then_response_is(&mut world, "A", "+COPS: 4,0,\"Android Virtual Operator\"");
    then_response_is(&mut world, "A", "OK");

    // 3. Wrong operator -> Fallback to automatic (cops_mode = 0)
    when_at_command_sent(&mut world, "A", "AT+COPS=4,0,\"Wrong Operator\"");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+COPS?");
    then_response_is(&mut world, "A", "+COPS: 0,0,\"Android Virtual Operator\"");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_csq_nr_technology() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // 1. Register and check standard LTE CSQ first
    when_at_command_sent(&mut world, "A", "AT+CREG=2");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CFUN=1");
    then_response_is(&mut world, "A", "OK");
    when_time_advances_ms(&mut world, 10);
    then_response_is(&mut world, "A", "+CREG: 1,\"2142\",\"0000B804\",7");
    then_response_is(
        &mut world,
        "A",
        "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,20,88,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647",
    );

    // 2. Change tech to NR (64 = bitmask for NR)
    when_at_command_sent(&mut world, "A", "AT+CTEC=64,\"21\"");
    then_response_is(&mut world, "A", "+CTEC: DONE");

    // Verify it triggers URC with CREG = 1 and new technology NR (11)
    then_response_is(&mut world, "A", "+CREG: 1,\"2142\",\"0000B804\",11");
    then_response_is(
        &mut world,
        "A",
        "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,88,2147483647,2147483647,2147483647,2147483647,2147483647",
    );
    then_response_is(&mut world, "A", "OK");

    // 3. Query CSQ, verify LTE fields are max and NR field (16) is 20!
    when_at_command_sent(&mut world, "A", "AT+CSQ");
    then_response_is(
        &mut world,
        "A",
        "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,88,2147483647,2147483647,2147483647,2147483647,2147483647",
    );
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_network_registration_radio_cycle_cfun_4() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // 1. Turn radio ON and check reactive URCs
    when_at_command_sent(&mut world, "A", "AT+CREG=1");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CFUN=1");
    then_response_is(&mut world, "A", "OK");

    when_time_advances_ms(&mut world, 10);
    then_response_is(&mut world, "A", "+CREG: 1");
    then_response_is(
        &mut world,
        "A",
        "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,20,88,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647",
    );

    // 2. Turn radio to Minimum Functionality (4) (drops registration and sends URC
    //    synchronously)
    when_at_command_sent(&mut world, "A", "AT+CFUN=4");
    then_response_is(&mut world, "A", "+CREG: 0");
    then_response_is(&mut world, "A", "OK");

    // Verify QueryRadioPower returns 4
    when_at_command_sent(&mut world, "A", "AT+CFUN?");
    then_response_is(&mut world, "A", "+CFUN: 4");
    then_response_is(&mut world, "A", "OK");

    // 3. Turn radio ON again and verify reactive URC triggers again!
    when_at_command_sent(&mut world, "A", "AT+CFUN=1");
    then_response_is(&mut world, "A", "OK");

    when_time_advances_ms(&mut world, 10);
    then_response_is(&mut world, "A", "+CREG: 1");
    then_response_is(
        &mut world,
        "A",
        "+CSQ: 99,99,2147483647,2147483647,2147483647,2147483647,2147483647,20,88,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647,2147483647",
    );
}
