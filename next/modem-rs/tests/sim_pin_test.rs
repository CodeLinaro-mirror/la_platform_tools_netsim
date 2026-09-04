// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{common::constants::*, steps::*, world::World};

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
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{INVALID_PIN}\""));
    then_response_is(&mut world, "A", "ERROR");

    // Second failed attempt
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{INVALID_PIN}\""));
    then_response_is(&mut world, "A", "ERROR");

    // Query retries
    when_at_command_sent(&mut world, "A", "AT+SPIC");
    then_response_is(&mut world, "A", &format!("+SPIC: {}", DEFAULT_PIN_RETRIES - 2));
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_pin_validation_errors() {
    let mut world = World::new();
    given_modem_with_locked_sim(&mut world, "A");

    // Enable verbose errors
    when_at_command_sent(&mut world, "A", "AT+CMEE=1");
    then_response_is(&mut world, "A", "OK");

    // 1. Invalid PIN length (too short) -> expect CME ERROR 16 (Incorrect Password)
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{TOO_SHORT_PIN}\""));
    then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PASSWORD);

    // 2. Invalid PIN length (too long) -> expect CME ERROR 16
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{TOO_LONG_PIN}\""));
    then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PASSWORD);

    // Transition to PukRequired to test PUK validation
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{INVALID_PIN}\""));
    then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PASSWORD);
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{INVALID_PIN}\""));
    then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PASSWORD);
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{INVALID_PIN}\""));
    then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PASSWORD);

    // Verify state is PUK required
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "+CPIN: SIM PUK");
    then_response_is(&mut world, "A", "OK");

    // 3. Invalid PUK length (too short) -> expect CME ERROR 16
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{TOO_SHORT_PUK}\",\"{LOCKED_PIN}\""));
    then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PASSWORD);

    // 4. Invalid new PIN length (too short) -> expect CME ERROR 16
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{TEST_PUK}\",\"{TOO_SHORT_PIN}\""));
    then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PASSWORD);

    // 5. Invalid new PIN length (too long) -> expect CME ERROR 16
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{TEST_PUK}\",\"{TOO_LONG_PIN}\""));
    then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PASSWORD);

    // 6. Missing new PIN -> expect CME ERROR 50 (Incorrect Parameters)
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{TEST_PUK}\""));
    then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PARAMETERS);
}

#[test]
fn test_query_sim_pin_facility_lock() {
    let mut world = World::new();
    given_modem(&mut world, "A"); // Default profile has PIN disabled

    // Query SC lock status -> should return +CLCK: 0 (disabled)
    when_at_command_sent(&mut world, "A", r#"AT+CLCK="SC",2"#);
    then_response_is(&mut world, "A", "+CLCK: 0");
    then_response_is(&mut world, "A", "OK");

    // Create a modem with locked SIM (PIN enabled)
    given_modem_with_locked_sim(&mut world, "B");

    // Query SC lock status -> should return +CLCK: 1 (enabled)
    when_at_command_sent(&mut world, "B", r#"AT+CLCK="SC",2"#);
    then_response_is(&mut world, "B", "+CLCK: 1");
    then_response_is(&mut world, "B", "OK");
}

#[test]
fn test_perm_blocked_sim_profile() {
    let mut world = World::new();
    given_modem_with_perm_blocked_sim(&mut world, "A");

    // Enable verbose errors
    when_at_command_sent(&mut world, "A", "AT+CMEE=1");
    then_response_is(&mut world, "A", "OK");

    // Query SIM status -> should return CME ERROR 13 (SimFailure)
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", CME_ERROR_SIM_FAILURE);

    // Attempting PIN entry should return CME ERROR 3 (OperationNotAllowed)
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{LOCKED_PIN}\""));
    then_response_is(&mut world, "A", CME_ERROR_OPERATION_NOT_ALLOWED);

    // Attempting PUK entry should return CME ERROR 3 (OperationNotAllowed)
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{TEST_PUK}\",\"{NEW_PIN}\""));
    then_response_is(&mut world, "A", CME_ERROR_OPERATION_NOT_ALLOWED);

    // Attempting password change should return CME ERROR 3
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CPWD=\"SC\",\"{LOCKED_PIN}\",\"{NEW_PIN}\""),
    );
    then_response_is(&mut world, "A", CME_ERROR_OPERATION_NOT_ALLOWED);
}

#[test]
fn test_puk_retries_exhaustion_transitions_to_perm_blocked() {
    let mut world = World::new();
    given_modem_with_locked_sim(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CMEE=1");
    then_response_is(&mut world, "A", "OK");

    // Exhaust PIN retries (3 attempts)
    for _ in 0..DEFAULT_PIN_RETRIES {
        when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{INVALID_PIN}\""));
        then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PASSWORD);
    }

    // Verify state transitioned to SIM PUK
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "+CPIN: SIM PUK");
    then_response_is(&mut world, "A", "OK");

    // Exhaust PUK retries (10 attempts)
    for _ in 0..DEFAULT_PUK_RETRIES {
        when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{INVALID_PUK}\",\"{NEW_PIN}\""));
        then_response_is(&mut world, "A", CME_ERROR_INCORRECT_PASSWORD);
    }

    // After exhausting PUK retries, SIM is PermBlocked:
    // AT+CPIN? returns CME ERROR 13.
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", CME_ERROR_SIM_FAILURE);

    // Any further unlock attempts must fail with CME ERROR 3
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{TEST_PUK}\",\"{NEW_PIN}\""));
    then_response_is(&mut world, "A", CME_ERROR_OPERATION_NOT_ALLOWED);
}
