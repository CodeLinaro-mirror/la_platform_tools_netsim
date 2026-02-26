use crate::{steps::*, world::World};

// Scenario: Two Modem End-to-End Flow (Call and SMS)
//   Given a modem "A"
//   And a modem "B" with number "12345"
//   When AT command "AT+CPIN?" is sent to "A"
//   Then response from "A" is "+CPIN: READY"
//   And response from "A" is "OK"
//   When AT command "AT+COPS?" is sent to "A"
//   Then response from "A" matches "+COPS: ..."
//   And response from "A" is "OK"
//   When AT command "ATD12345;" is sent to "A"
//   Then wait for connection (OK, RING, ATA, OK)
//   When AT command "AT+CLCC" is sent to "A"
//   Then response from "A" shows Active call
//   When AT command "ATH" is sent to "A"
//   Then response from "A" is "OK"
//   When AT command "AT+CLCC" is sent to "A"
//   Then response from "A" is "OK" (No calls)
//   When B sends SMS "hello" to A
//   Then A receives SMS
#[test]
fn test_two_modem_end_to_end_scenario() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    given_modem_with_number(&mut world, "B", "12345");

    // Power On Checks
    when_at_command_sent(&mut world, "A", "AT+CPIN?");
    then_response_is(&mut world, "A", "+CPIN: READY");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+COPS?");
    then_response_is(&mut world, "A", "+COPS: 0,0,\"Android Virtual Operator\"");
    then_response_is(&mut world, "A", "OK");

    // Make Call
    when_at_command_sent(&mut world, "A", "ATD12345;");
    then_response_is(&mut world, "A", "OK");

    then_wait_for_response_containing(&mut world, "B", "RING");
    when_at_command_sent(&mut world, "B", "ATA");
    then_response_is(&mut world, "B", "OK");
    then_wait_for_response_containing(&mut world, "A", "OK"); // Connected

    // Verify Active Call on A
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_wait_for_response_containing(&mut world, "A", "+CLCC: 1,0,0,0,0,\"12345\"");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // Hang Up
    when_at_command_sent(&mut world, "A", "ATH");
    then_response_is(&mut world, "A", "OK");

    // Verify Idle
    when_at_command_sent(&mut world, "A", "AT+CLCC");
    then_response_is(&mut world, "A", "OK");

    // Send SMS B -> A
    // B sends to A. A has no number.
    // Wait, integration_test.rs had: `manager.new_modem(modem_a_id, ...)`
    // And `manager.get_modem(modem_b_id).unwrap().
    // set_phone_number(constants::PHONE_NUMBER_A)` (which was "12345" assumed).
    // And A called B.
    // Then B sent SMS: `AT+CMGS=5`. Payload `hello`.
    // Where did B send it?
    // `AT+CMGS` prompts for address first? No.
    // Text mode: `AT+CMGS="addr"`.
    // PDU mode: `AT+CMGS=<length>`. PDU contains address.
    // Original test: `manager.send_at_command(modem_b_id, b"AT+CMGS=5\r\n");`
    // Then sent "hello".
    // "hello" is length 5.
    // Is this PDU or Text?
    // "hello" is NOT a valid PDU hex string.
    // Unless in text mode? B didn't set text mode. Default is PDU?
    // If PDU mode, "hello" (68656C6C6F) is invalid PDU.
    // BUT original test verified: `+CMT: ,5\r\nhello\r\n`.
    // This looks like `Text Mode` response? Or a custom simplied mode?
    // Netsim `sms_service.rs` might support raw text in PDU mode for testing?
    // Or previous test set text mode on B? No.

    // I will emulate EXACTLY what the original test did.
    // `AT+CMGS=5`. Then `hello`.
    // And expect `+CMT: ,5\r\nhello\r\n` on A.
    // This implies A received it.
    // Note: B calls A. But A has no number. PDU has no destination?
    // `ModemNetworkSimulator` likely broadcasts if no destination or handles
    // "loopback" or "default peer"? Harness `manager.get_peer(modem_id)`?

    when_at_command_sent(&mut world, "B", "AT+CMGS=5");
    then_response_is(&mut world, "B", "> ");

    // Send "hello" + Ctrl-Z.
    // I use hex bytes helper. "hello" -> 68656C6C6F. + 1A.
    let hello_hex = "68656C6C6F1A";
    when_hex_bytes_sent(&mut world, "B", hello_hex);

    then_wait_for_response_containing(&mut world, "B", "+CMGS: ");
    then_response_is(&mut world, "B", "OK");

    // Verify A received
    // Expect: "+CMT: ,5\r\nhello"
    let resp = then_wait_for_response_containing(&mut world, "A", "+CMT:");
    assert!(resp.contains("hello"));
}
