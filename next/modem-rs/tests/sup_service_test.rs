use crate::common::TestHarness;

#[test]
fn test_set_clip() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CLIP=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_call_waiting() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CCWA=1,1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_send_ussd() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CUSD=1,\"*123#\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CUSD: 0,\"OK\",15\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_cancel_ussd() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CUSD=2\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_call_forwarding() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CCFC=1,1,\"+1234567890\",145,20\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_query_clir() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CLIR?\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CLIR: 0,0\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_supp_service_notification() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CSSN=1,1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_facility_lock() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CLCK=\"SC\",1,\"1234\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}
