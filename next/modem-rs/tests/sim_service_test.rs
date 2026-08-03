// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{common::constants::*, steps::*, world::World};

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
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{TEST_PIN}\""));
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

    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{INVALID_PIN}\""));
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+SPIC");
    then_response_is(&mut world, "A", "+SPIC: 3");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cpin_set_no_quotes() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN={TEST_PIN}"));
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cpin_set_no_quotes_cmee() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CMEE=1");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN={TEST_PIN}"));
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
    then_response_is(&mut world, "A", TEST_IMSI);
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
    then_response_is(&mut world, "A", TEST_ICCID);
    then_response_is(&mut world, "A", "OK");
}

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
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{INVALID_PIN}\""));
    then_response_is(&mut world, "A", "ERROR");
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{INVALID_PIN}\""));
    then_response_is(&mut world, "A", "ERROR");
    when_at_command_sent(&mut world, "A", "AT+SPIC");
    then_response_is(&mut world, "A", &format!("+SPIC: {}", DEFAULT_PIN_RETRIES - 2));
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_pin_retry_counter_cpinr() {
    let mut world = World::new();
    given_modem_with_locked_sim(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CPINR=\"SIM PIN\"");
    then_response_is(
        &mut world,
        "A",
        &format!("+CPINR: \"SIM PIN\",{},{}", DEFAULT_PIN_RETRIES, DEFAULT_PIN_RETRIES),
    );
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{INVALID_PIN}\""));
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(&mut world, "A", "AT+CPINR=\"SIM PIN\"");
    then_response_is(
        &mut world,
        "A",
        &format!("+CPINR: \"SIM PIN\",{},{}", DEFAULT_PIN_RETRIES - 1, DEFAULT_PIN_RETRIES),
    );
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CPINR=\"SIM PUK\"");
    then_response_is(
        &mut world,
        "A",
        &format!("+CPINR: \"SIM PUK\",{},{}", DEFAULT_PUK_RETRIES, DEFAULT_PUK_RETRIES),
    );
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
    when_at_command_sent(&mut world, "A", &format!("AT+CCHO=\"{TEST_AID}\""));
    then_response_is(&mut world, "A", "1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Close Logical Channel
//   Given a modem "A"
//   When AT command 'AT+CCHO="{TEST_AID}"' is sent to "A"
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
    when_at_command_sent(&mut world, "A", &format!("AT+CCHO=\"{TEST_AID}\""));
    then_response_is(&mut world, "A", "1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CCHC=1");
    then_response_is(&mut world, "A", "+CCHC");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", &format!("AT+CGLA=1,10,\"{APDU_SELECT_EF_ICCID}\""));
    then_response_is(&mut world, "A", "ERROR");
}

// Scenario: Transmit Logical Channel
//   Given a modem "A"
//   When AT command 'AT+CCHO="{TEST_AID}"' is sent to "A"
//   Then response from "A" is "1"
//   And response from "A" is "OK"
//   When AT command 'AT+CGLA=1,10,"00A40004022FE2"' is sent to "A"
//   Then response from "A" is '+CGLA: 4,9000'
//   And response from "A" is "OK"
#[test]
fn test_transmit_logical_channel() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", &format!("AT+CCHO=\"{TEST_AID}\""));
    then_response_is(&mut world, "A", "1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", &format!("AT+CGLA=1,10,\"{APDU_SELECT_EF_ICCID}\""));
    then_response_is(&mut world, "A", &format!("+CGLA: {},{}", RESP_OK.len(), RESP_OK));
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

    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CPWD=\"SC\",\"{LOCKED_PIN}\",\"{NEW_PIN}\""),
    );
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{LOCKED_PIN}\""));
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{NEW_PIN}\""));
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
    then_response_is(&mut world, "A", "^MBAU: 0,0000000000000000,00000000");
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
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+REMOTEUPADATEPHONENUMBER=\"{TEST_PHONE_NUMBER}\""),
    );
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cmee_error_formatting_across_modes() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    let (id, _) = world.get_modem("A");
    world.manager.set_sim_status(id, false);
    then_wait_for_response_containing(&mut world, "A", "+CPIN: ABSENT");

    // Mode 0 (Disable): Returns standard ERROR
    when_at_command_sent(&mut world, "A", "AT+CMEE=0");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "ERROR");

    // Mode 1 (Numeric): Returns "+CME ERROR: 10" (SIM not inserted)
    when_at_command_sent(&mut world, "A", "AT+CMEE=1");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", CME_ERROR_SIM_NOT_INSERTED_NUMERIC);

    // Mode 2 (Verbose): Returns "+CME ERROR: SIM not inserted"
    when_at_command_sent(&mut world, "A", "AT+CMEE=2");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", CME_ERROR_SIM_NOT_INSERTED_VERBOSE);
}
// Scenario: Restricted SIM Access (AT+CRSM) read and write EF files
#[test]
fn test_sim_crsm_read_write() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A");

    // Read ICCID (2FE2)
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CRSM={},{},0,0,10", CRSM_READ_BINARY, EF_ICCID_DEC),
    ); // 176=Read Binary, 12258=0x2FE2
    then_response_is(&mut world, "A", &format!("+CRSM: 144,0,{}", TEST_ICCID_SWAPPED));
    then_response_is(&mut world, "A", "OK");

    // Write new ICCID via UPDATE BINARY (214 / 0xD6)
    when_at_command_sent(
        &mut world,
        "A",
        &format!(
            "AT+CRSM={},{},0,0,10,\"{}\"",
            CRSM_UPDATE_BINARY, EF_ICCID_DEC, NEW_ICCID_SWAPPED
        ),
    );
    then_response_is(&mut world, "A", "+CRSM: 144,0");
    then_response_is(&mut world, "A", "OK");

    // Verify it updated the ICCID
    when_at_command_sent(&mut world, "A", "AT+CICCID");
    then_response_is(&mut world, "A", NEW_ICCID);
    then_response_is(&mut world, "A", "OK");

    // Read IMSI (6F07) via AT+CRSM
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CRSM={},{},0,0,9", CRSM_READ_BINARY, EF_IMSI_DEC),
    ); // 28423=0x6F07
    then_response_is(&mut world, "A", &format!("+CRSM: 144,0,{}", TEST_IMSI_ENCODED)); // 123456789012345 encoded
    then_response_is(&mut world, "A", "OK");

    // Write IMSI via UPDATE BINARY
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CRSM={},{},0,0,9,\"{}\"", CRSM_UPDATE_BINARY, EF_IMSI_DEC, NEW_IMSI_ENCODED),
    );
    then_response_is(&mut world, "A", "+CRSM: 144,0");
    then_response_is(&mut world, "A", "OK");

    // Verify IMSI is updated via AT+CIMI
    when_at_command_sent(&mut world, "A", "AT+CIMI");
    then_response_is(&mut world, "A", NEW_IMSI);
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Update phone number and read MSISDN via AT+CRSM
#[test]
fn test_update_phone_number_and_read_msisdn() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A");

    // Set phone number via VENDOR command
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+REMOTEUPADATEPHONENUMBER=\"{}\"", TEST_MSISDN),
    );
    then_response_is(&mut world, "A", "OK");

    // Read EF_MSISDN (6F40) via AT+CRSM
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CRSM={},{},1,4,28", CRSM_READ_RECORD, EF_MSISDN_DEC),
    ); // 178=Read Record, 28480=0x6F40, record 1
    then_response_is(&mut world, "A", EXPECTED_MSISDN_CRSM_RESP);
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Test Carrier Privileges via PKCS15 files on logical channels
#[test]
fn test_carrier_privileges_logical_channels() {
    let mut world = World::new();
    given_modem_with_sim_type(&mut world, "A", 2);

    // Open channel
    when_at_command_sent(&mut world, "A", &format!("AT+CCHO=\"{TEST_AID}\""));
    then_response_is(&mut world, "A", "1");
    then_response_is(&mut world, "A", "OK");

    // Select file 4300
    when_at_command_sent(&mut world, "A", &format!("AT+CGLA=1,14,\"{APDU_SELECT_EF_4300}\""));
    then_response_is(&mut world, "A", RESP_SELECT_EF_4300);
    then_response_is(&mut world, "A", "OK");

    // Read file 4300
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,10,\"00B0000000\"");
    then_response_is(&mut world, "A", RESP_READ_EF_4300);
    then_response_is(&mut world, "A", "OK");

    // Select file 4318
    when_at_command_sent(&mut world, "A", &format!("AT+CGLA=1,14,\"{APDU_SELECT_EF_4318}\""));
    then_response_is(&mut world, "A", RESP_SELECT_EF_4318);
    then_response_is(&mut world, "A", "OK");

    // Read file 4318 (cert hashes)
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,10,\"00B0000000\"");
    then_response_is(&mut world, "A", RESP_READ_EF_4318);
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cpin_empty() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CPIN=\"\"");
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(&mut world, "A", "AT+CPINR=\"SIM PIN\"");
    then_response_is(
        &mut world,
        "A",
        &format!("+CPINR: \"SIM PIN\",{},{}", DEFAULT_PIN_RETRIES, DEFAULT_PIN_RETRIES),
    );
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_close_basic_logical_channel() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    // Closing channel 0 via MANAGE CHANNEL should return +CSIM status word (6A86)
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CSIM=10,\"{}\"", APDU_MANAGE_CHANNEL_CLOSE_CH0_CLA_80),
    );
    then_response_is(
        &mut world,
        "A",
        &format!("+CSIM: {},{}", RESP_ERROR_INCORRECT_PARAMS.len(), RESP_ERROR_INCORRECT_PARAMS),
    );
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_csim_unhandled_ins() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    // Sending unhandled INS via CSIM should return status word 6A86 rather than
    // +CME ERROR
    when_at_command_sent(&mut world, "A", &format!("AT+CSIM=10,\"{}\"", APDU_UNHANDLED_INS));
    then_response_is(
        &mut world,
        "A",
        &format!("+CSIM: {},{}", RESP_ERROR_INCORRECT_PARAMS.len(), RESP_ERROR_INCORRECT_PARAMS),
    );
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cgla_wrong_length() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", &format!("AT+CCHO=\"{TEST_AID}\""));
    then_response_is(&mut world, "A", "1");
    then_response_is(&mut world, "A", "OK");

    // Lc says 10 (0x0A) but only 4 bytes of data are passed ("00A400040A2FE2")
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,14,\"00A400040A2FE2\"");
    then_response_is(
        &mut world,
        "A",
        &format!("+CGLA: {},{}", RESP_ERROR_WRONG_LENGTH.len(), RESP_ERROR_WRONG_LENGTH),
    );
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_puk_unlocking() {
    let mut world = World::new();
    given_modem_with_locked_sim(&mut world, "A");

    // Enable CMEE to get detailed errors
    when_at_command_sent(&mut world, "A", "AT+CMEE=1");
    then_response_is(&mut world, "A", "OK");

    // Initial state should be SIM PIN required
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "+CPIN: SIM PIN");
    then_response_is(&mut world, "A", "OK");

    // SPIC initially shows 3 retries
    when_at_command_sent(&mut world, "A", "AT+SPIC");
    then_response_is(&mut world, "A", "+SPIC: 3");
    then_response_is(&mut world, "A", "OK");

    // Enter wrong PIN 1
    when_at_command_sent(&mut world, "A", "AT+CPIN=\"0000\"");
    then_response_is(&mut world, "A", "+CME ERROR: 16"); // Incorrect password

    when_at_command_sent(&mut world, "A", "AT+SPIC");
    then_response_is(&mut world, "A", "+SPIC: 2");
    then_response_is(&mut world, "A", "OK");

    // Enter wrong PIN 2
    when_at_command_sent(&mut world, "A", "AT+CPIN=\"0000\"");
    then_response_is(&mut world, "A", "+CME ERROR: 16");

    when_at_command_sent(&mut world, "A", "AT+SPIC");
    then_response_is(&mut world, "A", "+SPIC: 1");
    then_response_is(&mut world, "A", "OK");

    // Enter wrong PIN 3 -> Locks SIM, PUK required
    when_at_command_sent(&mut world, "A", "AT+CPIN=\"0000\"");
    then_response_is(&mut world, "A", "+CME ERROR: 16");

    // CPIN? should query SIM PUK
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "+CPIN: SIM PUK");
    then_response_is(&mut world, "A", "OK");

    // SPIC PIN retries should be 0
    when_at_command_sent(&mut world, "A", "AT+SPIC");
    then_response_is(&mut world, "A", "+SPIC: 0");
    then_response_is(&mut world, "A", "OK");

    // CPINR should show PUK retries (default 10)
    when_at_command_sent(&mut world, "A", "AT+CPINR=\"SIM PUK\"");
    then_response_is(&mut world, "A", "+CPINR: \"SIM PUK\",10,10");
    then_response_is(&mut world, "A", "OK");

    // Enter wrong PUK (should consume retry)
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{INVALID_PUK}\",\"{NEW_PIN}\""));
    then_response_is(&mut world, "A", "+CME ERROR: 16");

    when_at_command_sent(&mut world, "A", "AT+CPINR=\"SIM PUK\"");
    then_response_is(&mut world, "A", "+CPINR: \"SIM PUK\",9,10");
    then_response_is(&mut world, "A", "OK");

    // Enter correct PUK and set new PIN (puk1 is "12345678" in locked profile)
    when_at_command_sent(&mut world, "A", &format!("AT+CPIN=\"{TEST_PUK}\",\"{NEW_PIN}\""));
    then_response_is(&mut world, "A", "OK");

    // CPIN? should return READY
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "+CPIN: READY");
    then_response_is(&mut world, "A", "OK");

    // PIN retries should be reset to 3
    when_at_command_sent(&mut world, "A", "AT+SPIC");
    then_response_is(&mut world, "A", "+SPIC: 3");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_msisdn_update() {
    let mut world = World::new();
    given_modem_with_msisdn_in_fs(&mut world, "A");

    // 1. Read initial MSISDN (should be default "15555211001" assigned by
    //    simulator)
    when_at_command_sent(&mut world, "A", "AT+CRSM=178,28480,1,4,28");
    then_response_is(
        &mut world,
        "A",
        "+CRSM: 144,0,000000000000000000000000000007915155251100F1FFFFFFFFFFFF",
    );
    then_response_is(&mut world, "A", "OK");

    // 2. Update MSISDN to "15555215554" via APDU UPDATE RECORD (decimal 220)
    when_at_command_sent(
        &mut world,
        "A",
        "AT+CRSM=220,28480,1,4,28,\"000000000000000000000000000007915155255155F4FFFFFFFFFFFF\"",
    );
    then_response_is(&mut world, "A", "+CRSM: 144,0");
    then_response_is(&mut world, "A", "OK");

    // 3. Read it back and verify it matches
    when_at_command_sent(&mut world, "A", "AT+CRSM=178,28480,1,4,28");
    then_response_is(
        &mut world,
        "A",
        "+CRSM: 144,0,000000000000000000000000000007915155255155F4FFFFFFFFFFFF",
    );
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_update_record_invalid_record_number() {
    let mut world = World::new();
    given_modem_with_msisdn_in_fs(&mut world, "A");

    // UPDATE RECORD with invalid record number (0)
    when_at_command_sent(
        &mut world,
        "A",
        "AT+CRSM=220,28480,0,4,28,\"000000000000000000000000000007915155255155F4FFFFFFFFFFFF\"",
    );
    then_response_is(&mut world, "A", "+CRSM: 106,134"); // SW_INCORRECT_PARAMS (6A86)
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_update_record_wrong_length() {
    let mut world = World::new();
    given_modem_with_msisdn_in_fs(&mut world, "A");

    // UPDATE RECORD with wrong length
    when_at_command_sent(&mut world, "A", "AT+CRSM=220,28480,1,4,28,\"00000000000000000000\"");
    then_response_is(&mut world, "A", "+CRSM: 103,0"); // SW_WRONG_LENGTH (6700)
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_update_record_invalid_hex() {
    let mut world = World::new();
    given_modem_with_msisdn_in_fs(&mut world, "A");

    // UPDATE RECORD with invalid hex characters (e.g. 'G' at the end)
    // MSISDN record length is 28 bytes (56 hex characters)
    let invalid_hex = "000000000000000000000000000007915155255155F4FFFFFFFFFFFG";
    when_at_command_sent(&mut world, "A", &format!("AT+CRSM=220,28480,1,4,28,\"{invalid_hex}\""));
    then_response_is(&mut world, "A", "+CRSM: 106,134"); // SW_INCORRECT_PARAMS (6A86)
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_update_record_out_of_bounds() {
    let mut world = World::new();
    given_modem_with_msisdn_in_fs(&mut world, "A");

    // UPDATE RECORD for record out of bounds (record 5, only 1 exists)
    when_at_command_sent(
        &mut world,
        "A",
        "AT+CRSM=220,28480,5,4,28,\"000000000000000000000000000007915155255155F4FFFFFFFFFFFF\"",
    );
    then_response_is(&mut world, "A", "+CRSM: 106,136"); // SW_REFERENCED_DATA_NOT_FOUND (6A88)
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_transmit_logical_channel_errors() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Open channel 1 to be able to transmit on it
    when_at_command_sent(&mut world, "A", "AT+CCHO=\"A000000063504B43532D3135\"");
    then_response_is(&mut world, "A", "1");
    then_response_is(&mut world, "A", "OK");

    // 1. SW_FILE_NOT_FOUND (6A82) when selecting non-existent file
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,10,\"00A4000402FFFF\"");
    then_response_is(&mut world, "A", "+CGLA: 4,6A82");
    then_response_is(&mut world, "A", "OK");

    // 2. SW_INCORRECT_PARAMS (6A86) when manage channel has invalid p1
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,10,\"0070010000\"");
    then_response_is(&mut world, "A", "+CGLA: 4,6A86");
    then_response_is(&mut world, "A", "OK");

    // 3. SW_INS_NOT_SUPPORTED (6D00) when unknown instruction
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,10,\"00FF000000\"");
    then_response_is(&mut world, "A", "+CGLA: 4,6D00");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_sim_update_fplmn_and_mbdn() {
    let mut world = World::new();
    given_modem_with_fplmn_and_mbdn_in_fs(&mut world, "A");

    // 1. Test FPLMN (binary)
    // Update FPLMN
    when_at_command_sent(&mut world, "A", "AT+CRSM=214,28539,0,0,6,\"40F21040F220\"");
    then_response_is(&mut world, "A", "+CRSM: 144,0");
    then_response_is(&mut world, "A", "OK");

    // Read FPLMN and verify
    when_at_command_sent(&mut world, "A", "AT+CRSM=176,28539,0,0,12");
    then_response_is(&mut world, "A", "+CRSM: 144,0,40F21040F220FFFFFFFFFFFF");
    then_response_is(&mut world, "A", "OK");

    // 2. Test MBDN (record)
    let dummy_record = "A".repeat(76);
    // Update MBDN record 1
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CRSM=220,28615,1,4,38,\"{}\"", dummy_record),
    );
    then_response_is(&mut world, "A", "+CRSM: 144,0");
    then_response_is(&mut world, "A", "OK");

    // Read MBDN record 1 and verify
    when_at_command_sent(&mut world, "A", "AT+CRSM=178,28615,1,4,38");
    then_response_is(&mut world, "A", &format!("+CRSM: 144,0,{}", dummy_record));
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_sim_status_change_action() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Default status should be READY (SIM present)
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "+CPIN: READY");
    then_response_is(&mut world, "A", "OK");

    // Remove SIM
    when_sim_status_set(&mut world, "A", false);
    then_wait_for_response_containing(&mut world, "A", "+CPIN: ABSENT");

    // CPIN should fail
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "ERROR");

    // Re-insert SIM
    when_sim_status_set(&mut world, "A", true);
    then_wait_for_response_containing(&mut world, "A", "+CPIN: READY");

    // CPIN should work again
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "+CPIN: READY");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_update_binary_resize() {
    let mut world = World::new();
    given_modem_with_fplmn_and_mbdn_in_fs(&mut world, "A");

    // FPLMN size is 12 bytes (24 hex chars).
    // Write 4 bytes at offset 10 (total 14 bytes, requires resize).
    // P1=0, P2=10 (0x0A) -> offset 10.
    when_at_command_sent(&mut world, "A", "AT+CRSM=214,28539,0,10,4,\"11223344\"");
    then_response_is(&mut world, "A", "+CRSM: 144,0");
    then_response_is(&mut world, "A", "OK");

    // Read FPLMN (should now be 14 bytes)
    when_at_command_sent(&mut world, "A", "AT+CRSM=176,28539,0,0,14");
    then_response_is(&mut world, "A", "+CRSM: 144,0,FFFFFFFFFFFFFFFFFFFF11223344");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_update_record_non_record_file() {
    let mut world = World::new();
    given_modem_with_fplmn_and_mbdn_in_fs(&mut world, "A");

    // FPLMN (28539) is transparent. Try UPDATE RECORD (220) on it.
    when_at_command_sent(&mut world, "A", "AT+CRSM=220,28539,1,4,6,\"AABBCCDDEEFF\"");
    then_response_is(&mut world, "A", "+CRSM: 144,0");
    then_response_is(&mut world, "A", "OK");

    // Read it back (should be overwritten entirely)
    when_at_command_sent(&mut world, "A", "AT+CRSM=176,28539,0,0,6");
    then_response_is(&mut world, "A", "+CRSM: 144,0,AABBCCDDEEFF");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_update_missing_data() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Update FPLMN without data parameter.
    // If parser allows it, it should return +CRSM: 106,134.
    // If parser rejects it, it should return ERROR.
    // Let's test what it returns.
    when_at_command_sent(&mut world, "A", "AT+CRSM=214,28539,0,0,6");
    // We will assert +CRSM: 106,134 first. If it fails with parse error, we will
    // know.
    then_response_is(&mut world, "A", "+CRSM: 106,134");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_update_binary_invalid_hex() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Update FPLMN with invalid hex (e.g. 'G' at the end)
    when_at_command_sent(&mut world, "A", "AT+CRSM=214,28539,0,0,6,\"40F21040F22G\"");
    then_response_is(&mut world, "A", "+CRSM: 106,134"); // SW_INCORRECT_PARAMS (6A86)
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_update_file_not_found() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Try to update a file that is not in FS and not supported fallback (e.g.
    // 0x9999 = 39321)
    when_at_command_sent(&mut world, "A", "AT+CRSM=214,39321,0,0,1,\"00\"");
    then_response_is(&mut world, "A", "+CRSM: 106,130"); // SW_FILE_NOT_FOUND (6A82)
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cgla_close_closed_channel() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGLA=0,10,\"{APDU_MANAGE_CHANNEL_CLOSE_CH1}\""),
    );
    then_response_is(&mut world, "A", &format!("+CGLA: 4,{}", RESP_ERROR_NO_CHANNEL_AVAILABLE));
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cgla_validation_errors() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Too short APDU (less than 4 bytes)
    when_at_command_sent(&mut world, "A", "AT+CGLA=0,4,\"0070\"");
    then_response_is(&mut world, "A", "+CGLA: 4,6700");
    then_response_is(&mut world, "A", "OK");

    // Invalid hex data
    when_at_command_sent(&mut world, "A", "AT+CGLA=0,6,\"0070XX\"");
    then_response_is(&mut world, "A", "+CGLA: 4,6F00");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_csim_manage_channel_limits() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Open channel 1
    when_at_command_sent(&mut world, "A", "AT+CSIM=10,\"0070000000\"");
    then_response_is(&mut world, "A", "+CSIM: 6,019000");
    then_response_is(&mut world, "A", "OK");

    // Close channel 1 (verifies successful close path)
    when_at_command_sent(&mut world, "A", "AT+CSIM=10,\"0070800100\"");
    then_response_is(&mut world, "A", "+CSIM: 4,9000");
    then_response_is(&mut world, "A", "OK");

    // Open channel 1 again (should reuse index 1)
    when_at_command_sent(&mut world, "A", "AT+CSIM=10,\"0070000000\"");
    then_response_is(&mut world, "A", "+CSIM: 6,019000");
    then_response_is(&mut world, "A", "OK");

    // Open channel 2
    when_at_command_sent(&mut world, "A", "AT+CSIM=10,\"0070000000\"");
    then_response_is(&mut world, "A", "+CSIM: 6,029000");
    then_response_is(&mut world, "A", "OK");

    // Open channel 3
    when_at_command_sent(&mut world, "A", "AT+CSIM=10,\"0070000000\"");
    then_response_is(&mut world, "A", "+CSIM: 6,039000");
    then_response_is(&mut world, "A", "OK");

    // Try to open 5th channel (index 4, but limit is 4 channels (0-3))
    when_at_command_sent(&mut world, "A", "AT+CSIM=10,\"0070000000\"");
    then_response_is(&mut world, "A", "+CSIM: 4,6A81");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_csim_close_closed_channel_and_invalid_action() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Close closed channel 1
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CSIM=10,\"{APDU_MANAGE_CHANNEL_CLOSE_CH1}\""),
    );
    then_response_is(&mut world, "A", &format!("+CSIM: 4,{}", RESP_ERROR_NO_CHANNEL_AVAILABLE));
    then_response_is(&mut world, "A", "OK");

    // Close channel 0 (invalid operation)
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CSIM=10,\"{APDU_MANAGE_CHANNEL_CLOSE_CH0}\""),
    );
    then_response_is(&mut world, "A", &format!("+CSIM: 4,{}", RESP_ERROR_INCORRECT_PARAMS));
    then_response_is(&mut world, "A", "OK");

    // Close channel 4 (out of bounds)
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CSIM=10,\"{APDU_MANAGE_CHANNEL_CLOSE_CH4}\""),
    );
    then_response_is(&mut world, "A", &format!("+CSIM: 4,{}", RESP_ERROR_OUT_OF_BOUNDS));
    then_response_is(&mut world, "A", "OK");

    // Invalid action (P1=01)
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CSIM=10,\"{APDU_MANAGE_CHANNEL_INVALID_P1}\""),
    );
    then_response_is(&mut world, "A", &format!("+CSIM: 4,{}", RESP_ERROR_INCORRECT_PARAMS));
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cgla_manage_channel_open_and_close() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // 1. Open channel via CGLA on channel 0
    when_at_command_sent(&mut world, "A", &format!("AT+CGLA=0,10,\"{APDU_MANAGE_CHANNEL_OPEN}\""));
    // Should return channel ID "01" and SW_SUCCESS "9000" -> combined "019000",
    // length is 6 chars.
    then_response_is(&mut world, "A", "+CGLA: 6,019000");
    then_response_is(&mut world, "A", "OK");

    // 2. Verify we can transmit on the new channel 1 (e.g. SELECT EF_ICCID)
    when_at_command_sent(&mut world, "A", &format!("AT+CGLA=1,14,\"{APDU_SELECT_EF_ICCID}\""));
    then_response_is(&mut world, "A", "+CGLA: 4,9000");
    then_response_is(&mut world, "A", "OK");

    // 3. Close channel 1 via CGLA on channel 0
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGLA=0,10,\"{APDU_MANAGE_CHANNEL_CLOSE_CH1}\""),
    );
    then_response_is(&mut world, "A", "+CGLA: 4,9000");
    then_response_is(&mut world, "A", "OK");

    // 4. Verify channel 1 is now closed (transmitting on it should fail)
    when_at_command_sent(&mut world, "A", &format!("AT+CGLA=1,14,\"{APDU_SELECT_EF_ICCID}\""));
    then_response_is(&mut world, "A", "ERROR");
}

#[test]
fn test_cgla_close_invalid_channels() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Close channel 0 -> 6A86 (Incorrect params)
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGLA=0,10,\"{APDU_MANAGE_CHANNEL_CLOSE_CH0}\""),
    );
    then_response_is(&mut world, "A", &format!("+CGLA: 4,{}", RESP_ERROR_INCORRECT_PARAMS));
    then_response_is(&mut world, "A", "OK");

    // Close channel 4 -> 6A88 (Out of bounds)
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGLA=0,10,\"{APDU_MANAGE_CHANNEL_CLOSE_CH4}\""),
    );
    then_response_is(&mut world, "A", &format!("+CGLA: 4,{}", RESP_ERROR_OUT_OF_BOUNDS));
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_default_sim_profile_reads() {
    let mut world = World::new();
    // sim_type = 1 triggers the loading of the default production profile:
    // PROFILE_DEFAULT_XML
    given_modem_with_sim_type(&mut world, "A", 1);

    // 1. Verify CPIN status is READY (default profile does not lock by default)
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "+CPIN: READY");
    then_response_is(&mut world, "A", "OK");

    // 2. Query IMSI (AT+CIMI) -> parsed from USIM ADF
    when_at_command_sent(&mut world, "A", "AT+CIMI");
    then_response_is(&mut world, "A", "311740123456789");
    then_response_is(&mut world, "A", "OK");

    // 3. Query ICCID (AT+CICCID) -> parsed from CCID tag in EF_ICCID
    when_at_command_sent(&mut world, "A", "AT+CICCID");
    then_response_is(&mut world, "A", "89860318640220133897");
    then_response_is(&mut world, "A", "OK");

    // 4. Query ICCID via Restricted SIM Access (AT+CRSM) on EF_ICCID (12258 /
    //    0x2FE2)
    when_at_command_sent(&mut world, "A", "AT+CRSM=176,12258,0,0,10");
    then_response_is(&mut world, "A", "+CRSM: 144,0,98683081462002318379");
    then_response_is(&mut world, "A", "OK");

    // 5. Query Preferred Languages via Restricted SIM Access on EF_PL (12037 /
    //    0x2F05)
    when_at_command_sent(&mut world, "A", "AT+CRSM=176,12037,0,0,4");
    then_response_is(&mut world, "A", "+CRSM: 144,0,FFFFFFFF");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cts_profile_csim_and_status_reads() {
    let mut world = World::new();
    // sim_type = 2 triggers the loading of the CTS profile: PROFILE_CTS_XML
    given_modem_with_sim_type(&mut world, "A", 2);

    // 1. Verify AT+CSIM STATUS command query (80f2000000)
    // This command is explicitly mapped in the CSIM block of the CTS XML profile.
    // It returns the long Master File FCP template.
    when_at_command_sent(&mut world, "A", "AT+CSIM=10,\"80f2000000\"");
    then_response_is(
        &mut world,
        "A",
        "+CSIM: 110,62338202782183023F00A50C80016187010183040007DBF08A01058B062F0601020002C60C90016083010183010A83010D8102FFFF9000",
    );
    then_response_is(&mut world, "A", "OK");

    // 2. Verify AT+CSIM wrong length STATUS command query (80F20000)
    // This maps to the wrong length error response (6C35) configured in the XML.
    when_at_command_sent(&mut world, "A", "AT+CSIM=8,\"80F20000\"");
    then_response_is(&mut world, "A", "+CSIM: 4,6C35");
    then_response_is(&mut world, "A", "OK");

    // 3. Verify AT+CRSM STATUS command query (command 242 / 0xF2)
    // The simulator has a hardcoded override for APDU_STATUS (0xF2) to return the
    // MF FCP template:
    // "62338202782183023F00A50C80016187010183040007DBF08A01058B062F0601020002C60C90016083010183010A83010D8102FFFF"
    when_at_command_sent(&mut world, "A", "AT+CRSM=242,0,0,0,0");
    then_response_is(
        &mut world,
        "A",
        "+CRSM: 144,0,62338202782183023F00A50C80016187010183040007DBF08A01058B062F0601020002C60C90016083010183010A83010D8102FFFF",
    );
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cgla_apdu_parsing_cases() {
    let mut world = World::new();
    // sim_type = 1 triggers the loading of the default production profile:
    // PROFILE_DEFAULT_XML
    given_modem_with_sim_type(&mut world, "A", 1);

    // 1. Open Logical Channel 1 (Case 2 APDU: Le=00)
    when_at_command_sent(&mut world, "A", &format!("AT+CGLA=0,10,\"{APDU_MANAGE_CHANNEL_OPEN}\""));
    then_response_is(&mut world, "A", "+CGLA: 6,019000");
    then_response_is(&mut world, "A", "OK");

    // 2. Select EF_ICCID with FCP request (Case 4 APDU: Lc=02, Data=2FE2, Le=0C)
    // Send standard select to EF_ICCID: CLA=00 INS=A4 P1=00 P2=04 Lc=02 Data=2FE2
    // Le=0C
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,16,\"00A40004022FE20C\"");
    then_response_is(&mut world, "A", "+CGLA: 4,9000");
    then_response_is(&mut world, "A", "OK");

    // 3. Read Binary Part (Case 2 APDU, Le > 0)
    // Read 5 bytes of selected EF_ICCID: CLA=00 INS=B0 P1=00 P2=00 Le=05
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,10,\"00B0000005\"");
    then_response_is(&mut world, "A", "+CGLA: 14,98683081469000");
    then_response_is(&mut world, "A", "OK");

    // 4. Read Binary Full (Case 2 APDU, Le = 0)
    // Read all bytes of selected EF_ICCID: CLA=00 INS=B0 P1=00 P2=00 Le=00
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,10,\"00B0000000\"");
    then_response_is(&mut world, "A", "+CGLA: 24,986830814620023183799000");
    then_response_is(&mut world, "A", "OK");

    // 5. Invalid Length APDU (6 bytes, but Lc=2 which expects 7 bytes)
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,12,\"00B000000201\"");
    then_response_is(&mut world, "A", "+CGLA: 4,6700");
    then_response_is(&mut world, "A", "OK");

    // 6. Close Channel 1 (Case 2 APDU: Le=00)
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGLA=0,10,\"{APDU_MANAGE_CHANNEL_CLOSE_CH1}\""),
    );
    then_response_is(&mut world, "A", "+CGLA: 4,9000");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cgla_invalid_channel_index() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Enable verbose errors
    when_at_command_sent(&mut world, "A", "AT+CMEE=2");
    then_response_is(&mut world, "A", "OK");

    // Transmit on invalid channel index 4
    when_at_command_sent(&mut world, "A", "AT+CGLA=4,10,\"00A40004022FE2\"");
    then_response_is(&mut world, "A", "+CME ERROR: invalid index");
}

#[test]
fn test_apdu_read_binary_offset_out_of_bounds() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A"); // ICCID size is 10

    // Read at offset 11
    when_at_command_sent(&mut world, "A", "AT+CRSM=176,12258,0,11,1");
    then_response_is(&mut world, "A", "+CRSM: 106,134");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_read_binary_range_out_of_bounds() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A"); // ICCID size is 10

    // Read 5 bytes at offset 8 (ends at 13, past size 10)
    when_at_command_sent(&mut world, "A", "AT+CRSM=176,12258,0,8,5");
    then_response_is(&mut world, "A", "+CRSM: 103,0");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_read_binary_at_end_of_file() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A"); // ICCID size is 10

    // Read 0 bytes at offset 10 (exactly at end of file)
    when_at_command_sent(&mut world, "A", "AT+CRSM=176,12258,0,10,0");
    then_response_is(&mut world, "A", "+CRSM: 144,0,");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_read_record_file_not_found() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A");

    // Read record 1 of non-existent file 9999
    when_at_command_sent(&mut world, "A", "AT+CRSM=178,9999,1,4,28");
    then_response_is(&mut world, "A", "+CRSM: 106,130");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_read_record_transparent_file() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A"); // ICCID is transparent

    // Try to read record 1 of ICCID
    when_at_command_sent(&mut world, "A", "AT+CRSM=178,12258,1,4,10");
    then_response_is(&mut world, "A", "+CRSM: 106,130");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_read_record_index_zero() {
    let mut world = World::new();
    given_modem_with_msisdn_in_fs(&mut world, "A"); // MSISDN is record file

    // Read record 0 of MSISDN
    when_at_command_sent(&mut world, "A", "AT+CRSM=178,28480,0,4,28");
    then_response_is(&mut world, "A", "+CRSM: 106,136");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_apdu_read_record_out_of_bounds() {
    let mut world = World::new();
    given_modem_with_msisdn_in_fs(&mut world, "A");

    // Read record 5 of MSISDN (only 1 exists)
    when_at_command_sent(&mut world, "A", "AT+CRSM=178,28480,5,4,28");
    then_response_is(&mut world, "A", "+CRSM: 106,136");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_even_length_imsi_processing() {
    let mut world = World::new();
    let xml = r#"<IccProfile>
        <MF>
            <ADF aid="A0000000871002FF86FF0389FFFFFFFF">
                <EF id="6F07" structure="transparent">
                    <CIMI>31174012345678</CIMI>
                </EF>
            </ADF>
        </MF>
    </IccProfile>"#;
    given_modem_with_xml_profile(&mut world, "A", xml);

    // 1. Check IMSI readout via AT+CIMI
    when_at_command_sent(&mut world, "A", "AT+CIMI");
    then_response_is(&mut world, "A", "31174012345678");
    then_response_is(&mut world, "A", "OK");

    // 2. Check low-level BCD encoding and padding in EF_IMSI file
    when_at_command_sent(&mut world, "A", "AT+CRSM=176,28423,0,0,9");
    then_response_is(&mut world, "A", "+CRSM: 144,0,0831114710325476F8");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cgla_apdu_write_cases() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A"); // ICCID is transparent, MSISDN is linear fixed

    // 1. Open Logical Channel 1
    when_at_command_sent(&mut world, "A", &format!("AT+CGLA=0,10,\"{APDU_MANAGE_CHANNEL_OPEN}\""));
    then_response_is(&mut world, "A", "+CGLA: 6,019000");
    then_response_is(&mut world, "A", "OK");

    // 2. Select EF_ICCID
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,16,\"00A40004022FE20C\"");
    then_response_is(&mut world, "A", "+CGLA: 4,9000");
    then_response_is(&mut world, "A", "OK");

    // 3. Write Binary (UPDATE BINARY Case 3: CLA=00 INS=D6 P1=00 P2=05 Lc=05
    //    Data=1122334455)
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,20,\"00D60005051122334455\"");
    then_response_is(&mut world, "A", "+CGLA: 4,9000");
    then_response_is(&mut world, "A", "OK");

    // 4. Read Binary Back to verify state (Le=05)
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,10,\"00B0000505\"");
    then_response_is(&mut world, "A", "+CGLA: 14,11223344559000");
    then_response_is(&mut world, "A", "OK");

    // 5. Select EF_MSISDN (28480 = 0x6F40)
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,16,\"00A40004026F400C\"");
    then_response_is(&mut world, "A", "+CGLA: 4,9000");
    then_response_is(&mut world, "A", "OK");

    // 6. Write Record (UPDATE RECORD Case 3: CLA=00 INS=DC P1=01 P2=04 Lc=1C
    //    Data=28-bytes)
    // MSISDN record length is 28 bytes (56 hex characters) in the default mock
    // layout
    let new_record_hex = "0000000000000000000000000000079181F2FF00000FFFFFFFFFFFFF";
    when_at_command_sent(&mut world, "A", &format!("AT+CGLA=1,66,\"00DC01041C{new_record_hex}\""));
    then_response_is(&mut world, "A", "+CGLA: 4,9000");
    then_response_is(&mut world, "A", "OK");

    // 7. Read Record Back to verify state (Le=1C)
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,10,\"00B201041C\"");
    then_response_is(&mut world, "A", &format!("+CGLA: 60,{new_record_hex}9000"));
    then_response_is(&mut world, "A", "OK");

    // 8. Close Channel 1
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGLA=0,10,\"{APDU_MANAGE_CHANNEL_CLOSE_CH1}\""),
    );
    then_response_is(&mut world, "A", "+CGLA: 4,9000");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cgla_invalid_file_and_channel_handling() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A");

    // Try to select a non-existent file ID (e.g. 0xFFFF)
    when_at_command_sent(&mut world, "A", "AT+CGLA=0,16,\"00A4000402FFFF0C\"");
    then_response_is(&mut world, "A", "+CGLA: 4,6A82");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_csim_select_and_unhandled_ins() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // 1. SELECT command via CSIM returns success
    when_at_command_sent(&mut world, "A", "AT+CSIM=14,\"00A40004022FE2\"");
    then_response_is(&mut world, "A", "+CSIM: 4,9000");
    then_response_is(&mut world, "A", "OK");

    // 2. Unhandled INS via CSIM returns SW_INCORRECT_PARAMS (6A86) as baseline
    when_at_command_sent(&mut world, "A", "AT+CSIM=10,\"00FE000000\"");
    then_response_is(&mut world, "A", "+CSIM: 4,6A86");
    then_response_is(&mut world, "A", "OK");

    // 3. APDU with mismatched Lc length via CSIM returns SW_WRONG_LENGTH (6700)
    when_at_command_sent(&mut world, "A", "AT+CSIM=14,\"00A400040A1122\"");
    then_response_is(&mut world, "A", "+CSIM: 4,6700");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_csim_invalid_channel_rejection() {
    let mut world = World::new();
    given_modem_with_sim_type(&mut world, "A", 1);

    // 1. Send command with CLA 40 (Format 2, channel 4) -> out of bounds (max 3
    //    supported)
    // Expect SW_CLASS_NOT_SUPPORTED (6E00)
    when_at_command_sent(&mut world, "A", "AT+CSIM=10,\"40B0000005\"");
    then_response_is(&mut world, "A", "+CSIM: 4,6E00");
    then_response_is(&mut world, "A", "OK");

    // 2. Send command with CLA 01 (Format 1, channel 1) which is closed
    // Expect SW_CLASS_NOT_SUPPORTED (6E00)
    when_at_command_sent(&mut world, "A", "AT+CSIM=10,\"01B0000005\"");
    then_response_is(&mut world, "A", "+CSIM: 4,6E00");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_lenient_cla_rejection() {
    let mut world = World::new();
    given_modem_with_sim_type(&mut world, "A", 1);

    // 1. Open channel 1
    when_at_command_sent(&mut world, "A", "AT+CGLA=0,10,\"0070000001\"");
    then_response_is(&mut world, "A", "+CGLA: 6,019000");
    then_response_is(&mut world, "A", "OK");

    // 2. Open channel 2
    when_at_command_sent(&mut world, "A", "AT+CGLA=0,10,\"0070000001\"");
    then_response_is(&mut world, "A", "+CGLA: 6,029000");
    then_response_is(&mut world, "A", "OK");

    // 3. Transmit on channel 1 with APDU specifying channel 2 (CLA 02)
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,10,\"02F2000000\"");
    then_response_is(&mut world, "A", "+CGLA: 4,6E00");
    then_response_is(&mut world, "A", "OK");
}
