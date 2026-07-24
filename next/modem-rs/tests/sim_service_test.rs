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

#[test]
fn test_cpin_set_no_quotes() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CPIN=1234");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cpin_set_no_quotes_cmee() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CMEE=1");
    then_response_is(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", "AT+CPIN=1234");
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
    when_at_command_sent(&mut world, "A", "AT+CPIN=\"0000\"");
    then_response_is(&mut world, "A", "ERROR");
    when_at_command_sent(&mut world, "A", "AT+CPIN=\"0000\"");
    then_response_is(&mut world, "A", "ERROR");
    when_at_command_sent(&mut world, "A", "AT+SPIC");
    then_response_is(&mut world, "A", "+SPIC: 1");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_pin_retry_counter_cpinr() {
    let mut world = World::new();
    given_modem_with_locked_sim(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CPINR=\"SIM PIN\"");
    then_response_is(&mut world, "A", "+CPINR: \"SIM PIN\",3,3");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CPIN=\"0000\"");
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(&mut world, "A", "AT+CPINR=\"SIM PIN\"");
    then_response_is(&mut world, "A", "+CPINR: \"SIM PIN\",2,3");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CPINR=\"SIM PUK\"");
    then_response_is(&mut world, "A", "+CPINR: \"SIM PUK\",10,10");
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
    when_at_command_sent(&mut world, "A", "AT+CCHO=\"A000000063504B43532D3135\"");
    then_response_is(&mut world, "A", "1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Close Logical Channel
//   Given a modem "A"
//   When AT command 'AT+CCHO="A000000063504B43532D3135"' is sent to "A"
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
    when_at_command_sent(&mut world, "A", "AT+CCHO=\"A000000063504B43532D3135\"");
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
//   When AT command 'AT+CCHO="A000000063504B43532D3135"' is sent to "A"
//   Then response from "A" is "1"
//   And response from "A" is "OK"
//   When AT command 'AT+CGLA=1,10,"00A40004022FE2"' is sent to "A"
//   Then response from "A" is '+CGLA: 4,9000'
//   And response from "A" is "OK"
#[test]
fn test_transmit_logical_channel() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CCHO=\"A000000063504B43532D3135\"");
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
// Scenario: Restricted SIM Access (AT+CRSM) read and write EF files
#[test]
fn test_sim_crsm_read_write() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A");

    // Read ICCID (2FE2)
    when_at_command_sent(&mut world, "A", "AT+CRSM=176,12258,0,0,10"); // 176=Read Binary, 12258=0x2FE2
    then_response_is(&mut world, "A", "+CRSM: 144,0,89012345678901234567");
    then_response_is(&mut world, "A", "OK");

    // Write new ICCID via UPDATE BINARY (214 / 0xD6)
    when_at_command_sent(&mut world, "A", "AT+CRSM=214,12258,0,0,10,\"89012608640220133897\"");
    then_response_is(&mut world, "A", "+CRSM: 144,0");
    then_response_is(&mut world, "A", "OK");

    // Verify it updated the ICCID
    when_at_command_sent(&mut world, "A", "AT+CICCID");
    then_response_is(&mut world, "A", "89012608640220133897");
    then_response_is(&mut world, "A", "OK");

    // Read IMSI (6F07) via AT+CRSM
    when_at_command_sent(&mut world, "A", "AT+CRSM=176,28423,0,0,9"); // 28423=0x6F07
    then_response_is(&mut world, "A", "+CRSM: 144,0,081932547698103254"); // 123456789012345 encoded
    then_response_is(&mut world, "A", "OK");

    // Write IMSI via UPDATE BINARY
    when_at_command_sent(&mut world, "A", "AT+CRSM=214,28423,0,0,9,\"083901621032547698\"");
    then_response_is(&mut world, "A", "+CRSM: 144,0");
    then_response_is(&mut world, "A", "OK");

    // Verify IMSI is updated via AT+CIMI
    when_at_command_sent(&mut world, "A", "AT+CIMI");
    then_response_is(&mut world, "A", "310260123456789");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Update phone number and read MSISDN via AT+CRSM
#[test]
fn test_update_phone_number_and_read_msisdn() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A");

    // Set phone number via VENDOR command
    when_at_command_sent(&mut world, "A", "AT+REMOTEUPADATEPHONENUMBER=\"15555215554\"");
    then_response_is(&mut world, "A", "OK");

    // Read EF_MSISDN (6F40) via AT+CRSM
    when_at_command_sent(&mut world, "A", "AT+CRSM=178,28480,1,4,28"); // 178=Read Record, 28480=0x6F40, record 1
    then_response_is(
        &mut world,
        "A",
        "+CRSM: 144,0,000000000000000000000000000007915155255155F4FFFFFFFFFFFF",
    );
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Test Carrier Privileges via PKCS15 files on logical channels
#[test]
fn test_carrier_privileges_logical_channels() {
    let mut world = World::new();
    given_modem_with_sim_type(&mut world, "A", 2);

    // Open channel
    when_at_command_sent(&mut world, "A", "AT+CCHO=\"A000000063504B43532D3135\"");
    then_response_is(&mut world, "A", "1");
    then_response_is(&mut world, "A", "OK");

    // Select file 4300
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,14,\"00A40004024300\"");
    then_response_is(
        &mut world,
        "A",
        "+CGLA: 76,62228202412183024300A503C001408A01058B066F0601010001800201DC810201EE88009000",
    );
    then_response_is(&mut world, "A", "OK");

    // Read file 4300
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,10,\"00B0000000\"");
    then_response_is(
        &mut world,
        "A",
        "+CGLA: 516,30088200300404024310301AA0120410A000000476416E64726F696443545340300404024311301AA0120410A000000476416E64726F696443545341300404024312301AA0120410A000000476416E64726F696443545342300404024313301AA0120410A000000476416E64726F696443545343300404024314301AA0120410A000000476416E64726F696443545344300404024315301AA0120410A000000476416E64726F696443545345300404024316301AA0120410A000000476416E64726F6964435453463004040243173010A0080406FFFFFFFFFFFF300404024318301AA0120410A000000476416E64726F696443545347300404024313301AA0129000",
    );
    then_response_is(&mut world, "A", "OK");

    // Select file 4318
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,14,\"00A40004024318\"");
    then_response_is(
        &mut world,
        "A",
        "+CGLA: 76,62228202412183024318A503C001408A01058B066F0601010001800200188102002A88009000",
    );
    then_response_is(&mut world, "A", "OK");

    // Read file 4318 (cert hashes)
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,10,\"00B0000000\"");
    then_response_is(
        &mut world,
        "A",
        "+CGLA: 124,3016041461ED377E85D386A8DFEE6B864BD85B0BFAA5AF8130220420CE7B2B47AE2B7552C8F92CC29124279883041FB623A5F194A82C9BF15D492AA09000",
    );
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cpin_empty() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CPIN=\"\"");
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(&mut world, "A", "AT+CPINR=\"SIM PIN\"");
    then_response_is(&mut world, "A", "+CPINR: \"SIM PIN\",3,3");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_close_basic_logical_channel() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    // Closing channel 0 via MANAGE CHANNEL should return +CSIM status word (6A86)
    when_at_command_sent(&mut world, "A", "AT+CSIM=10,\"8070800000\"");
    then_response_is(&mut world, "A", "+CSIM: 4,6A86");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_csim_unhandled_ins() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    // Sending unhandled INS via CSIM should return status word 6A86 rather than
    // +CME ERROR
    when_at_command_sent(&mut world, "A", "AT+CSIM=10,\"00FA000000\"");
    then_response_is(&mut world, "A", "+CSIM: 4,6A86");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_cgla_wrong_length() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CCHO=\"A000000063504B43532D3135\"");
    then_response_is(&mut world, "A", "1");
    then_response_is(&mut world, "A", "OK");

    // Lc says 10 (0x0A) but only 4 bytes of data are passed ("00A400040A2FE2")
    when_at_command_sent(&mut world, "A", "AT+CGLA=1,14,\"00A400040A2FE2\"");
    then_response_is(&mut world, "A", "+CGLA: 4,6700");
    then_response_is(&mut world, "A", "OK");
}
