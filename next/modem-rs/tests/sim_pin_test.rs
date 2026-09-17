// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{common::constants::*, world::World};

// Scenario: Verify PIN Retry Counter
//   Given a modem "A"
//   When AT command 'AT+CPIN="0000"' is sent to "A"
//   Then response from "A" is "ERROR"
//   When AT command 'AT+CPIN="0000"' is sent to "A"
//   Then response from "A" is "ERROR"
//   When AT command 'AT+CPINR="SIM PIN"' is sent to "A"
//   Then response from "A" is "+CPINR: \"SIM PIN\",1,3"
//   And response from "A" is "OK"
#[test]
fn test_pin_retry_counter() {
    let mut world = World::new();
    world.given_modem_with_locked_sim("A");

    // First failed attempt
    world.send_and_expect_error("A", &format!("AT+CPIN=\"{INVALID_PIN}\""), "ERROR");

    // Second failed attempt
    world.send_and_expect_error("A", &format!("AT+CPIN=\"{INVALID_PIN}\""), "ERROR");

    // Query retries
    world.send_and_expect(
        "A",
        "AT+CPINR=\"SIM PIN\"",
        &[&format!("+CPINR: \"SIM PIN\",{},{DEFAULT_PIN_RETRIES}", DEFAULT_PIN_RETRIES - 2), "OK"],
    );
}

#[test]
fn test_pin_validation_errors() {
    let mut world = World::new();
    world.given_modem_with_locked_sim("A");

    // Enable verbose errors
    world.send_and_expect_ok("A", "AT+CMEE=1");

    // 1. Invalid PIN length (too short) -> expect CME ERROR 16 (Incorrect Password)
    world.send_and_expect_error(
        "A",
        &format!("AT+CPIN=\"{TOO_SHORT_PIN}\""),
        CME_ERROR_INCORRECT_PASSWORD,
    );

    // 2. Invalid PIN length (too long) -> expect CME ERROR 16
    world.send_and_expect_error(
        "A",
        &format!("AT+CPIN=\"{TOO_LONG_PIN}\""),
        CME_ERROR_INCORRECT_PASSWORD,
    );

    // 3. Non-ASCII PIN (multi-byte UTF-8) -> expect CME ERROR 16
    world.send_and_expect_error("A", "AT+CPIN=\"\u{00E9}\u{00E9}\"", CME_ERROR_INCORRECT_PASSWORD);

    // Transition to PukRequired to test PUK validation
    world.send_and_expect_error(
        "A",
        &format!("AT+CPIN=\"{INVALID_PIN}\""),
        CME_ERROR_INCORRECT_PASSWORD,
    );
    world.send_and_expect_error(
        "A",
        &format!("AT+CPIN=\"{INVALID_PIN}\""),
        CME_ERROR_INCORRECT_PASSWORD,
    );
    world.send_and_expect_error(
        "A",
        &format!("AT+CPIN=\"{INVALID_PIN}\""),
        CME_ERROR_INCORRECT_PASSWORD,
    );

    // Verify state is PUK required
    world.send_and_expect("A", "AT+CPIN?", &["+CPIN: SIM PUK", "OK"]);

    // 3. Invalid PUK length (too short) -> expect CME ERROR 16
    world.send_and_expect_error(
        "A",
        &format!("AT+CPIN=\"{TOO_SHORT_PUK}\",\"{LOCKED_PIN}\""),
        CME_ERROR_INCORRECT_PASSWORD,
    );

    // 4. Invalid new PIN length (too short) -> expect CME ERROR 16
    world.send_and_expect_error(
        "A",
        &format!("AT+CPIN=\"{TEST_PUK}\",\"{TOO_SHORT_PIN}\""),
        CME_ERROR_INCORRECT_PASSWORD,
    );

    // 5. Invalid new PIN length (too long) -> expect CME ERROR 16
    world.send_and_expect_error(
        "A",
        &format!("AT+CPIN=\"{TEST_PUK}\",\"{TOO_LONG_PIN}\""),
        CME_ERROR_INCORRECT_PASSWORD,
    );

    // 6. Missing new PIN -> expect CME ERROR 50 (Incorrect Parameters)
    world.send_and_expect_error(
        "A",
        &format!("AT+CPIN=\"{TEST_PUK}\""),
        CME_ERROR_INCORRECT_PARAMETERS,
    );
}

#[test]
fn test_query_sim_pin_facility_lock() {
    let mut world = World::new();
    world.given_modem("A"); // Default profile has PIN disabled

    // Query SC lock status -> should return +CLCK: 0 (disabled)
    world.send_and_expect("A", r#"AT+CLCK="SC",2"#, &["+CLCK: 0", "OK"]);

    // Create a modem with locked SIM (PIN enabled)
    world.given_modem_with_locked_sim("B");

    // Query SC lock status -> should return +CLCK: 1 (enabled)
    world.send_and_expect("B", r#"AT+CLCK="SC",2"#, &["+CLCK: 1", "OK"]);
}

#[test]
fn test_perm_blocked_sim_profile() {
    let mut world = World::new();
    world.given_modem_with_perm_blocked_sim("A");

    // Enable verbose errors
    world.send_and_expect_ok("A", "AT+CMEE=1");

    // Query SIM status -> should return CME ERROR 13 (SimFailure)
    world.send_and_expect_error("A", "AT+CPIN?", CME_ERROR_SIM_FAILURE);

    // Attempting PIN entry should return CME ERROR 3 (OperationNotAllowed)
    world.send_and_expect_error(
        "A",
        &format!("AT+CPIN=\"{LOCKED_PIN}\""),
        CME_ERROR_OPERATION_NOT_ALLOWED,
    );

    // Attempting PUK entry should return CME ERROR 3 (OperationNotAllowed)
    world.send_and_expect_error(
        "A",
        &format!("AT+CPIN=\"{TEST_PUK}\",\"{NEW_PIN}\""),
        CME_ERROR_OPERATION_NOT_ALLOWED,
    );

    // Attempting password change should return CME ERROR 3
    world.send_and_expect_error(
        "A",
        &format!("AT+CPWD=\"SC\",\"{LOCKED_PIN}\",\"{NEW_PIN}\""),
        CME_ERROR_OPERATION_NOT_ALLOWED,
    );
}

#[test]
fn test_puk_retries_exhaustion_transitions_to_perm_blocked() {
    let mut world = World::new();
    world.given_modem_with_locked_sim("A");

    world.send_and_expect_ok("A", "AT+CMEE=1");

    // Exhaust PIN retries (3 attempts)
    for _ in 0..DEFAULT_PIN_RETRIES {
        world.send_and_expect_error(
            "A",
            &format!("AT+CPIN=\"{INVALID_PIN}\""),
            CME_ERROR_INCORRECT_PASSWORD,
        );
    }

    // Verify state transitioned to SIM PUK
    world.send_and_expect("A", "AT+CPIN?", &["+CPIN: SIM PUK", "OK"]);

    // Exhaust PUK retries (10 attempts)
    for _ in 0..DEFAULT_PUK_RETRIES {
        world.send_and_expect_error(
            "A",
            &format!("AT+CPIN=\"{INVALID_PUK}\",\"{NEW_PIN}\""),
            CME_ERROR_INCORRECT_PASSWORD,
        );
    }

    // After exhausting PUK retries, SIM is PermBlocked:
    // AT+CPIN? returns CME ERROR 13.
    world.send_and_expect_error("A", "AT+CPIN?", CME_ERROR_SIM_FAILURE);

    // Any further unlock attempts must fail with CME ERROR 3
    world.send_and_expect_error(
        "A",
        &format!("AT+CPIN=\"{TEST_PUK}\",\"{NEW_PIN}\""),
        CME_ERROR_OPERATION_NOT_ALLOWED,
    );
}
