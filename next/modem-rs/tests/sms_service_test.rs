use crate::{steps::*, world::World};

const PDU_HEX: &str = "0011000B915155255155F40000AA01F0";
const PDU_HEX_WITH_CTRL_Z: &str = "0011000B915155255155F40000AA01F01A";

// Scenario: Send SMS PDU Mode
//   Given a modem "A"
//   When AT command "AT+CMGS=14" is sent to "A"
//   Then response from "A" is "> "
//   When hex bytes are sent to "A"
//   Then response from "A" matches "+CMGS: "
//   And response from "A" is "OK"
#[test]
fn test_cmgs() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CMGS=14");
    then_response_is(&mut world, "A", "> ");

    when_hex_bytes_sent(&mut world, "A", PDU_HEX_WITH_CTRL_Z);
    then_response_contains(&mut world, "A", "+CMGS: ");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Store and Read SMS
//   Given a modem "A"
//   When AT command "AT+CMGW=16" is sent to "A"
//   Then response from "A" is "> "
//   When hex bytes are sent to "A"
//   Then response from "A" contains "+CMGW: "
//   And response from "A" is "OK"
//   When AT command "AT+CMGR=1" is sent to "A"
//   Then response from "A" starts with "+CMGR: 0,,16" and contains PDU
//   And response from "A" is "OK"
#[test]
fn test_store_and_read_sms() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // PDU Length = 16
    when_at_command_sent(&mut world, "A", "AT+CMGW=16");
    then_response_is(&mut world, "A", "> ");

    when_hex_bytes_sent(&mut world, "A", PDU_HEX_WITH_CTRL_Z);
    then_response_contains(&mut world, "A", "+CMGW: 1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CMGR=1");
    // Response buffer contains PDU_HEX
    then_response_contains(&mut world, "A", &format!("+CMGR: 0,,16\r\n{}", PDU_HEX));
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Store and Read SMS on SIM
//   Given a modem "A"
//   When AT command 'AT+CPMS="SM","SM","SM"' is sent to "A"
//   Then response from "A" is "OK" (Wait, CPMS returns +CPMS: ... then OK
// usually?)   (Original test code cleared responses after CPMS. Some handlers
// might just return OK if setting?)   (test_set_preferred_message_storage
// expects +CPMS: ... then OK on QUERY. On SET?)   (Original test:
// harness.send_at_command(CPMS); harness.get_responses(). (Ignored output).
//   (Wait, CPMS set command returns +CPMS: <used>,<total>... then OK? Check
// test_set_preferred_message_storage in old file?
//   (No, test_set_preferred_message_storage sends SET, clears, then QUERIES.
//   (So SET command output is ignored or unknown. Standard says +CPMS: ... OK.)
//   (I'll consume responses until OK).
#[test]
fn test_store_and_read_sms_on_sim() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CPMS=\"SM\",\"SM\",\"SM\"");
    // Consume responses until OK or use a lenient check
    // Assuming +CPMS response comes before OK
    // then_response_contains(&mut world, "A", "+CPMS:");
    // then_response_is(&mut world, "A", "OK");
    // But original code ignored the content. I'll just wait for "OK".
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CMGW=16");
    then_response_is(&mut world, "A", "> ");

    when_hex_bytes_sent(&mut world, "A", PDU_HEX_WITH_CTRL_Z);
    then_response_contains(&mut world, "A", "+CMGW: 1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CMGR=1");
    then_response_contains(&mut world, "A", &format!("+CMGR: 0,,16\r\n{}", PDU_HEX));
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Delete SMS
//   Given a modem "A"
//   When AT command 'AT+CPMS="SM","SM","SM"' is sent to "A"
//   Then wait for OK
//   When AT command "AT+CMGW=16" is sent to "A"
//   Then response from "A" is "> "
//   When hex bytes are sent to "A"
//   Then response from "A" contains "+CMGW: 1"
//   And response from "A" is "OK"
//   When AT command "AT+CMGD=1" is sent to "A"
//   Then response from "A" is "OK"
//   When AT command "AT+CMGR=1" is sent to "A"
//   Then response from "A" is "ERROR"
#[test]
fn test_delete_sms() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CPMS=\"SM\",\"SM\",\"SM\"");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CMGW=16");
    then_response_is(&mut world, "A", "> ");

    when_hex_bytes_sent(&mut world, "A", PDU_HEX_WITH_CTRL_Z);
    then_response_contains(&mut world, "A", "+CMGW: 1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CMGD=1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CMGR=1");
    then_response_is(&mut world, "A", "ERROR");
}

// Scenario: Delete SMS on SIM
//   (Same logic just ensure CPMS set correctly)
#[test]
fn test_delete_sms_on_sim() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CPMS=\"SM\",\"SM\",\"SM\"");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CMGW=16");
    then_response_is(&mut world, "A", "> ");

    when_hex_bytes_sent(&mut world, "A", PDU_HEX_WITH_CTRL_Z);
    then_response_contains(&mut world, "A", "+CMGW: 1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CMGD=1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CMGR=1");
    then_response_is(&mut world, "A", "ERROR");
}

// Scenario: SMS New Message Acknowledgement
//   Given a modem "A"
//   When AT command "AT+CNMA" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_cnma() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CNMA");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set SMS Message Format
//   Given a modem "A"
//   When AT command "AT+CMGF=1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_cmgf() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CMGF=1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Broadcast Configuration
//   Given a modem "A"
//   When AT command 'AT+CSCB=0,"1,2,3","4,5,6"' is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_broadcast_config() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CSCB=0,\"1,2,3\",\"4,5,6\"");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set SMSC Address
//   Given a modem "A"
//   When AT command 'AT+CSCA="+1234567890"' is sent to "A"
//   Then response from "A" is "OK"
//   When AT command "AT+CSCA?" is sent to "A"
//   Then response from "A" is '+CSCA: "+1234567890",145'
//   And response from "A" is "OK"
#[test]
fn test_smsc_address() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CSCA=\"+1234567890\"");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CSCA?");
    then_response_is(&mut world, "A", "+CSCA: \"+1234567890\",145");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Remote SMS
//   Given a modem "A"
//   When AT command 'AT+REMOTESMS="0011000B915155255155F40000AA01F0"' is sent
// to "A"   Then response from "A" is "OK"
#[test]
fn test_remote_sms() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+REMOTESMS=\"0011000B915155255155F40000AA01F0\"");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set Preferred Message Storage
//   Given a modem "A"
//   When AT command 'AT+CPMS="SM","SM","SM"' is sent to "A"
//   Then wait for OK
//   When AT command "AT+CPMS?" is sent to "A"
//   Then response from "A" matches "+CPMS: ..."
//   And response from "A" is "OK"
#[test]
fn test_set_preferred_message_storage() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CPMS=\"SM\",\"SM\",\"SM\"");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CPMS?");
    then_response_contains(&mut world, "A", "+CPMS: \"SM\",0,255,\"SM\",0,255,\"SM\",0,255");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Send SMS Text Mode
//   Given a modem "A"
//   And a modem "B" with number "12345"
//   When AT command "AT+CMGF=1" is sent to "A"
//   Then response from "A" is "OK"
//   When AT command 'AT+CMGS="12345"' is sent to "A"
//   Then response from "A" is "> "
//   When hex bytes "48656C6C6F1A" (Hello^Z) are sent to "A"
//   Then response from "A" matches "+CMGS: "
//   And response from "A" is "OK"
//   Then response from "B" starts with "+CMT: "12345""
//   And response from "B" ends with "Hello"
#[test]
fn test_send_sms_text_mode() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    given_modem_with_number(&mut world, "B", "12345");

    when_at_command_sent(&mut world, "A", "AT+CMGF=1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CMGS=\"12345\"");
    then_response_is(&mut world, "A", "> ");

    // Hello + Ctrl-Z
    let hello_hex = "48656C6C6F1A";
    when_hex_bytes_sent(&mut world, "A", hello_hex);

    then_response_contains(&mut world, "A", "+CMGS: ");
    then_response_is(&mut world, "A", "OK");

    // Verify reception on B
    // CMT unsolicited
    let resp = then_wait_for_response_containing(&mut world, "B", "+CMT: \"12345\"");
    assert!(resp.contains("Hello"), "Response should contain message body 'Hello'");
}
