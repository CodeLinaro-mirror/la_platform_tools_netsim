// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{steps::*, world::World};

// Scenario: Query PIN Status
//   Given a modem "A"
//   When AT command "AT+CPIN?" is sent to "A"
//   Then response from "A" is "+CPIN: READY"
//   And response from "A" is "OK"
#[test]
fn test_cpin_query() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "+CPIN: READY");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Enter PIN
//   Given a modem "A"
//   When AT command 'AT+CPIN="1234"' is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_cpin_set() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CPIN=\"1234\"");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: CPIN in READY State
//   Given a modem "A"
//   When AT command "AT+SPIC" is sent to "A"
//   Then response from "A" is "+SPIC: 3"
//   And response from "A" is "OK"
//   When AT command 'AT+CPIN="0000"' is sent to "A"
//   Then response from "A" is "OK"
//   When AT command "AT+SPIC" is sent to "A"
//   Then response from "A" is "+SPIC: 3"
//   And response from "A" is "OK"
#[test]
fn test_cpin_in_ready_state_does_not_consume_retry() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+SPIC");
    then_response_is(&mut world, "A", "+SPIC: 3");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CPIN=\"0000\"");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+SPIC");
    then_response_is(&mut world, "A", "+SPIC: 3");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Request IMSI
//   Given a modem "A"
//   When AT command "AT+CIMI" is sent to "A"
//   Then response from "A" is "123456789012345"
//   And response from "A" is "OK"
#[test]
fn test_cimi() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CIMI");
    then_response_is(&mut world, "A", "123456789012345");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Request ICCID
//   Given a modem "A"
//   When AT command "AT+CICCID" is sent to "A"
//   Then response from "A" is "89012345678901234567"
//   And response from "A" is "OK"
#[test]
fn test_cicc() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CICCID");
    then_response_is(&mut world, "A", "89012345678901234567");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Open Logical Channel
//   Given a modem "A"
//   When AT command 'AT+CCHO="1234"' is sent to "A"
//   Then response from "A" is "1"
//   And response from "A" is "OK"
#[test]
fn test_open_logical_channel() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CCHO=\"1234\"");
    then_response_is(&mut world, "A", "1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Close Logical Channel
//   Given a modem "A"
//   When AT command 'AT+CCHO="1234"' is sent to "A"
//   Then response from "A" is "1"
//   And response from "A" is "OK"
//   When AT command "AT+CCHC=1" is sent to "A"
//   Then response from "A" is "+CCHC"
//   And response from "A" is "OK"
//   When AT command 'AT+CGLA=1,10,"00A40004022FE2"' is sent to "A"
//   Then response from "A" is "ERROR"
#[test]
fn test_close_logical_channel() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CCHO=\"1234\"");
    then_response_is(&mut world, "A", "1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CCHC=1");
    then_response_is(&mut world, "A", "+CCHC");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGLA=1,10,\"00A40004022FE2\"");
    then_response_is(&mut world, "A", "ERROR");
}

// Scenario: Transmit Logical Channel
//   Given a modem "A"
//   When AT command 'AT+CCHO="1234"' is sent to "A"
//   Then response from "A" is "1"
//   And response from "A" is "OK"
//   When AT command 'AT+CGLA=1,10,"00A40004022FE2"' is sent to "A"
//   Then response from "A" is '+CGLA: 4,9000'
//   And response from "A" is "OK"
#[test]
fn test_transmit_logical_channel() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CCHO=\"1234\"");
    then_response_is(&mut world, "A", "1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGLA=1,10,\"00A40004022FE2\"");
    then_response_is(&mut world, "A", "+CGLA: 4,9000");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Change Password
//   Given a modem "A" with locked SIM
//   When AT command 'AT+CPWD="SC","1111","4321"' is sent to "A"
//   Then response from "A" is "OK"
//   When AT command 'AT+CPIN="1111"' is sent to "A"
//   Then response from "A" is "ERROR"
//   When AT command 'AT+CPIN="4321"' is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_change_password() {
    let mut world = World::new();
    given_modem_with_locked_sim(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CPWD=\"SC\",\"1111\",\"4321\"");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CPIN=\"1111\"");
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(&mut world, "A", "AT+CPIN=\"4321\"");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set CDMA Subscription Source
//   Given a modem "A"
//   When AT command "AT+CCSS=1" is sent to "A"
//   Then response from "A" is "OK"
//   When AT command "AT+CCSS?" is sent to "A"
//   Then response from "A" is "+CCSS: 1"
//   And response from "A" is "OK"
#[test]
fn test_set_cdma_subscription_source() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CCSS=1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CCSS?");
    then_response_is(&mut world, "A", "+CCSS: 1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set CDMA Roaming Preference
//   Given a modem "A"
//   When AT command "AT+WRMP=1" is sent to "A"
//   Then response from "A" is "OK"
//   When AT command "AT+WRMP?" is sent to "A"
//   Then response from "A" is "+WRMP: 1"
//   And response from "A" is "OK"
#[test]
fn test_set_cdma_roaming_preference() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+WRMP=1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+WRMP?");
    then_response_is(&mut world, "A", "+WRMP: 1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: SIM Authentication
//   Given a modem "A"
//   When AT command 'AT+MBAU="some_data"' is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_sim_authentication() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+MBAU=\"some_data\"");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Update Phone Number
//   Given a modem "A"
//   When AT command 'AT+REMOTEUPADATEPHONENUMBER="1234567890"' is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_update_phone_number() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+REMOTEUPADATEPHONENUMBER=\"1234567890\"");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cmee_error_formatting_across_modes() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    let (id, _) = world.get_modem("A");
    world.manager.set_sim_status(id, false);

    // Mode 0 (Disable): Returns standard ERROR
    when_at_command_sent(&mut world, "A", "AT+CMEE=0");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "ERROR");

    // Mode 1 (Numeric): Returns "+CME ERROR: 10" (SIM not inserted)
    when_at_command_sent(&mut world, "A", "AT+CMEE=1");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "+CME ERROR: 10");

    // Mode 2 (Verbose): Returns "+CME ERROR: SIM not inserted"
    when_at_command_sent(&mut world, "A", "AT+CMEE=2");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "+CME ERROR: SIM not inserted");
}
