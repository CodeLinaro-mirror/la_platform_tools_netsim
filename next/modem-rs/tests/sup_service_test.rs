// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{common::constants::*, steps::*, world::World};

// Scenario: Set and Query CLIP (Calling Line Identification Presentation)
//   Given a modem "A"
//   When AT command "AT+CLIP?" is sent to "A"
//   Then response from "A" is "+CLIP: 0,1"
//   And response from "A" is "OK"
//   When AT command "AT+CLIP=1" is sent to "A"
//   Then response from "A" is "OK"
//   When AT command "AT+CLIP?" is sent to "A"
//   Then response from "A" is "+CLIP: 1,1"
//   And response from "A" is "OK"
//   When AT command "AT+CLIP=0" is sent to "A"
//   Then response from "A" is "OK"
//   When AT command "AT+CLIP?" is sent to "A"
//   Then response from "A" is "+CLIP: 0,1"
//   And response from "A" is "OK"
#[test]
fn test_set_and_query_clip() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Default query
    when_at_command_sent(&mut world, "A", "AT+CLIP?");
    then_response_is(&mut world, "A", "+CLIP: 0,1");
    then_response_is(&mut world, "A", "OK");

    // Enable CLIP
    when_at_command_sent(&mut world, "A", "AT+CLIP=1");
    then_response_is(&mut world, "A", "OK");

    // Query enabled state
    when_at_command_sent(&mut world, "A", "AT+CLIP?");
    then_response_is(&mut world, "A", "+CLIP: 1,1");
    then_response_is(&mut world, "A", "OK");

    // Disable CLIP
    when_at_command_sent(&mut world, "A", "AT+CLIP=0");
    then_response_is(&mut world, "A", "OK");

    // Query disabled state
    when_at_command_sent(&mut world, "A", "AT+CLIP?");
    then_response_is(&mut world, "A", "+CLIP: 0,1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set Call Waiting
//   Given a modem "A"
//   When AT command "AT+CCWA=1,1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_set_call_waiting() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CCWA=1,1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Send USSD
//   Given a modem "A"
//   When AT command 'AT+CUSD=1,"*123#"' is sent to "A"
//   Then response from "A" is '+CUSD: 0,"OK",15'
//   And response from "A" is "OK"
#[test]
fn test_send_ussd() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", &format!("AT+CUSD=1,\"{TEST_USSD}\""));
    then_response_is(&mut world, "A", r#"+CUSD: 0,"OK",15"#);
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Cancel USSD
//   Given a modem "A"
//   When AT command "AT+CUSD=2" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_cancel_ussd() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CUSD=2");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Call Forwarding
//   Given a modem "A"
//   When AT command 'AT+CCFC=1,1,"+1234567890",145,20' is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_call_forwarding() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CCFC=1,1,\"{TEST_SMSC}\",{},20", TOA_INTERNATIONAL),
    );
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Query CLIR
//   Given a modem "A"
//   When AT command "AT+CLIR?" is sent to "A"
//   Then response from "A" is "+CLIR: 0,0"
//   And response from "A" is "OK"
#[test]
fn test_query_clir() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CLIR?");
    then_response_is(&mut world, "A", "+CLIR: 0,1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Supplementary Service Notification
//   Given a modem "A"
//   When AT command "AT+CSSN=1,1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_supp_service_notification() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CSSN=1,1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set Facility Lock
//   Given a modem "A"
//   When AT command 'AT+CLCK="SC",1,"1234"' is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_set_facility_lock() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", &format!("AT+CLCK=\"SC\",1,\"{TEST_PIN}\""));
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_query_facility_lock() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", r#"AT+CLCK="FD",2"#);
    then_response_is(&mut world, "A", "+CLCK: 0");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_call_forward_utility() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CMEE=1");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CCFCU=0,3,2,{},\"{CALL_PEER_ALT}\",1,\"\",\"\",,1", TOA_NATIONAL),
    );
    then_response_is(&mut world, "A", CME_ERROR_OPERATION_NOT_SUPPORTED);
}

#[test]
fn test_call_forward_utility_cmee_disabled() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CMEE=0");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CCFCU=0,3,2,{},\"{CALL_PEER_ALT}\",1,\"\",\"\",,1", TOA_NATIONAL),
    );
    then_response_is(&mut world, "A", "ERROR");
}

#[test]
fn test_disable_facility_lock() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Enable lock first
    when_at_command_sent(&mut world, "A", &format!("AT+CLCK=\"SC\",1,\"{TEST_PIN}\""));
    then_response_is(&mut world, "A", "OK");

    // Disable lock with correct password
    when_at_command_sent(&mut world, "A", &format!("AT+CLCK=\"SC\",0,\"{TEST_PIN}\""));
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_facility_lock_errors() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Enable CMEE
    when_at_command_sent(&mut world, "A", "AT+CMEE=1");
    then_response_is(&mut world, "A", "OK");

    // 1. Disable lock with wrong password -> expect CME ERROR 16
    when_at_command_sent(&mut world, "A", r#"AT+CLCK="SC",0,"wrong""#);
    then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PASSWORD);

    // 2. Disable lock with missing password -> expect CME ERROR 16
    when_at_command_sent(&mut world, "A", r#"AT+CLCK="SC",0"#);
    then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PASSWORD);
}

#[test]
fn test_facility_lock_puk_required() {
    let mut world = World::new();
    given_modem_with_locked_sim(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CMEE=1");
    then_response_is(&mut world, "A", "OK");

    // Locked SIM starts in PinRequired.
    // Enter wrong PIN 3 times to trigger PukRequired (correct is "1111")
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{INVALID_PIN}\""));
    then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PASSWORD);
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{INVALID_PIN}\""));
    then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PASSWORD);
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{INVALID_PIN}\""));
    then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PASSWORD);

    // Verify it is PukRequired
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "+CPIN: SIM PUK");
    then_response_is(&mut world, "A", "OK");

    // Try to disable lock -> expect CME ERROR 12 (SIM PUK required)
    when_at_command_sent(&mut world, "A", &format!("AT+CLCK=\"SC\",0,\"{LOCKED_PIN}\""));
    then_response_is(&mut world, "A", CME_ERROR_SIM_PUK_REQUIRED);
}

#[test]
fn test_ccwa_lifecycle() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // 1. Query CCWA with default class, should be disabled (status 0, class 7)
    when_at_command_sent(&mut world, "A", "AT+CCWA=1,2");
    then_response_is(&mut world, "A", "+CCWA: 0,7");
    then_response_is(&mut world, "A", "OK");

    // 2. Query CCWA with specific class (returns status 0, class 1 for voice)
    when_at_command_sent(&mut world, "A", "AT+CCWA=1,2,1");
    then_response_is(&mut world, "A", "+CCWA: 0,1");
    then_response_is(&mut world, "A", "OK");

    // 3. Enable CCWA for voice (class 1)
    when_at_command_sent(&mut world, "A", "AT+CCWA=1,1,1");
    then_response_is(&mut world, "A", "OK");

    // 4. Query CCWA again, should be enabled (status 1, class 1)
    when_at_command_sent(&mut world, "A", "AT+CCWA=1,2,1");
    then_response_is(&mut world, "A", "+CCWA: 1,1");
    then_response_is(&mut world, "A", "OK");

    // 4a. Query CCWA with default class (7), should return only active classes
    // (status 1, class 1 for voice)
    when_at_command_sent(&mut world, "A", "AT+CCWA=1,2");
    then_response_is(&mut world, "A", "+CCWA: 1,1");
    then_response_is(&mut world, "A", "OK");

    // 4b. Query CCWA for data (class 2), should be disabled (status 0, class 2)
    when_at_command_sent(&mut world, "A", "AT+CCWA=1,2,2");
    then_response_is(&mut world, "A", "+CCWA: 0,2");
    then_response_is(&mut world, "A", "OK");

    // 4c. Enable CCWA for Voice+Data+Fax (class 7)
    when_at_command_sent(&mut world, "A", "AT+CCWA=1,1,7");
    then_response_is(&mut world, "A", "OK");

    // 4d. Query CCWA for Voice (class 1), should be enabled (status 1, class 1)
    when_at_command_sent(&mut world, "A", "AT+CCWA=1,2,1");
    then_response_is(&mut world, "A", "+CCWA: 1,1");
    then_response_is(&mut world, "A", "OK");

    // 4e. Query CCWA for Data (class 2), should be enabled (status 1, class 2)
    when_at_command_sent(&mut world, "A", "AT+CCWA=1,2,2");
    then_response_is(&mut world, "A", "+CCWA: 1,2");
    then_response_is(&mut world, "A", "OK");

    // 4f. Query CCWA with default class (7), should return all active classes
    // (status 1, class 1, 2, 4)
    when_at_command_sent(&mut world, "A", "AT+CCWA=1,2");
    then_response_is(&mut world, "A", "+CCWA: 1,1");
    then_response_is(&mut world, "A", "+CCWA: 1,2");
    then_response_is(&mut world, "A", "+CCWA: 1,4");
    then_response_is(&mut world, "A", "OK");

    // 5. Disable CCWA for all (class 7)
    when_at_command_sent(&mut world, "A", "AT+CCWA=1,0,7");
    then_response_is(&mut world, "A", "OK");

    // 6. Query CCWA again for voice, should be disabled (status 0, class 1)
    when_at_command_sent(&mut world, "A", "AT+CCWA=1,2,1");
    then_response_is(&mut world, "A", "+CCWA: 0,1");
    then_response_is(&mut world, "A", "OK");

    // 6b. Query CCWA again for data, should be disabled (status 0, class 2)
    when_at_command_sent(&mut world, "A", "AT+CCWA=1,2,2");
    then_response_is(&mut world, "A", "+CCWA: 0,2");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_set_clir() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Set CLIR to 1 (presentation restricted)
    when_at_command_sent(&mut world, "A", "AT+CLIR=1");
    then_response_is(&mut world, "A", "OK");

    // Query CLIR, should be 1,1 (since it is provisioned)
    when_at_command_sent(&mut world, "A", "AT+CLIR?");
    then_response_is(&mut world, "A", "+CLIR: 1,1");
    then_response_is(&mut world, "A", "OK");
}
