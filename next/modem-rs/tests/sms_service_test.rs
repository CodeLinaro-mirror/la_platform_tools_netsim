mod common;
use common::TestHarness;

#[test]
fn test_cmgs() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CMGS=14\r\n");
    harness.send_at_command(b"0011000B915155255155F40000AA01F0\x1a");

    let responses = harness.get_responses();
    assert_eq!(responses.len(), 3);
    assert_eq!(responses[0], b"> \r\n");
    assert!(responses[1].starts_with(b"+CMGS: "));
    assert_eq!(responses[2], b"OK\r\n");
}

#[test]
fn test_store_and_read_sms() {
    let harness = TestHarness::new();
    let pdu = hex::decode("0011000B915155255155F40000AA01F0").unwrap();
    harness.send_at_command(format!("AT+CMGW={}\r\n", pdu.len()).as_bytes());
    let mut command = pdu.to_vec();
    command.push(0x1a);
    harness.send_at_command(&command);
    harness.get_responses(); // Clear responses
    harness.send_at_command(b"AT+CMGR=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(
        responses[0],
        format!("+CMGR: 0,,{}\r\n{}\r\n", pdu.len(), hex::encode_upper(&pdu)).as_bytes()
    );
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_store_and_read_sms_on_sim() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CPMS=\"SM\",\"SM\",\"SM\"\r\n");
    harness.get_responses(); // Clear responses

    let pdu = hex::decode("0011000B915155255155F40000AA01F0").unwrap();
    harness.send_at_command(format!("AT+CMGW={}\r\n", pdu.len()).as_bytes());
    let mut command = pdu.to_vec();
    command.push(0x1a);
    harness.send_at_command(&command);
    harness.get_responses(); // Clear responses

    harness.send_at_command(b"AT+CMGR=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(
        responses[0],
        format!("+CMGR: 0,,{}\r\n{}\r\n", pdu.len(), hex::encode_upper(&pdu)).as_bytes()
    );
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_delete_sms() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CPMS=\"SM\",\"SM\",\"SM\"\r\n");
    harness.get_responses(); // Clear responses

    let pdu = hex::decode("0011000B915155255155F40000AA01F0").unwrap();
    harness.send_at_command(format!("AT+CMGW={}\r\n", pdu.len()).as_bytes());
    let mut command = pdu.to_vec();
    command.push(0x1a);
    harness.send_at_command(&command);
    harness.get_responses(); // Clear responses

    harness.send_at_command(b"AT+CMGD=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");

    harness.send_at_command(b"AT+CMGR=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"ERROR\r\n");
}

#[test]
fn test_delete_sms_on_sim() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CPMS=\"SM\",\"SM\",\"SM\"\r\n");
    harness.get_responses(); // Clear responses

    let pdu = hex::decode("0011000B915155255155F40000AA01F0").unwrap();
    harness.send_at_command(format!("AT+CMGW={}\r\n", pdu.len()).as_bytes());
    let mut command = pdu.to_vec();
    command.push(0x1a);
    harness.send_at_command(&command);
    harness.get_responses(); // Clear responses

    harness.send_at_command(b"AT+CMGD=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");

    harness.send_at_command(b"AT+CMGR=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"ERROR\r\n");
}

#[test]
fn test_cnma() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CNMA\r\n");

    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_cmgf() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CMGF=1\r\n");

    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_broadcast_config() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CSCB=0,\"1,2,3\",\"4,5,6\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_smsc_address() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CSCA=\"+1234567890\"\r\n");
    harness.get_responses(); // Clear responses
    harness.send_at_command(b"AT+CSCA?\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CSCA: \"+1234567890\",145\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_remote_sms() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+REMOTESMS=\"0011000B915155255155F40000AA01F0\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_preferred_message_storage() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CPMS=\"SM\",\"SM\",\"SM\"\r\n");
    harness.get_responses(); // Clear responses
    harness.send_at_command(b"AT+CPMS?\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CPMS: \"SM\",0,255,\"SM\",0,255,\"SM\",0,255\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_send_sms_text_mode() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CMGF=1\r\n");
    harness.get_responses(); // Clear responses

    let peer_modem = harness.manager.get_peer(harness.modem_id).unwrap();
    peer_modem.set_phone_number("12345");

    harness.send_at_command(b"AT+CMGS=\"12345\"\r\n");
    harness.send_at_command(b"Hello\x1a");

    let responses = harness.get_responses();
    assert_eq!(responses.len(), 3);
    assert_eq!(responses[0], b"> \r\n");
    assert!(responses[1].starts_with(b"+CMGS: "));
    assert_eq!(responses[2], b"OK\r\n");

    let peer_handler = harness.get_modem_handler(peer_modem.id).unwrap();
    let responses = peer_handler.get_responses();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].starts_with(b"+CMT: \"12345\""));
    assert!(responses[0].ends_with(b"\r\nHello\r\n"));
}
