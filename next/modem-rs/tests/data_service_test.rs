mod common;
use common::TestHarness;

#[test]
fn test_set_and_query_quality_of_service_minimum() {
    let harness = TestHarness::new();
    // First, define the context we are about to modify
    harness.send_at_command(b"AT+CGDCONT=1,\"IP\",\"test\"\r\n");
    harness.get_responses(); // Clear setup response
    harness.send_at_command(b"AT+CGEQMIN=1,2,3,4,5,6\r\n");
    harness.get_responses(); // Clear setup response
    harness.send_at_command(b"AT+CGEQMIN?\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CGEQMIN: 1,2,3,4,5,6\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_set_pdp_context_activate() {
    let harness = TestHarness::new();
    // First, define the context we are about to modify
    harness.send_at_command(b"AT+CGDCONT=1,\"IP\",\"test\"\r\n");
    harness.get_responses(); // Clear setup response
    harness.send_at_command(b"AT+CGACT=1,1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_update_physical_channel_configs() {
    let harness = TestHarness::new();
    let modem = harness.manager.get_modem(harness.modem_id).unwrap();
    modem.data_service.on_update_physical_channel_configs(&modem);
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"+CGEV: NW PDN DEACT 1\r\n");
}

#[test]
fn test_read_dynamic_param() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CGSCONTRDP=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CGSCONTRDP: 1, 5, 1500, 300000, 300000, 300000, 300000\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_query_pdp_context() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CGDCONT?\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_and_query_quality_of_service_requested() {
    let harness = TestHarness::new();
    // First, define the context we are about to modify
    harness.send_at_command(b"AT+CGDCONT=1,\"IP\",\"test\"\r\n");
    harness.get_responses(); // Clear setup response
    harness.send_at_command(b"AT+CGEQREQ=1,2,3,4,5,6\r\n");
    harness.get_responses(); // Clear setup response
    harness.send_at_command(b"AT+CGEQREQ?\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CGEQREQ: 1,2,3,4,5,6\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_set_and_query_quality_of_service_minimum_gprs() {
    let harness = TestHarness::new();
    // First, define the context we are about to modify
    harness.send_at_command(b"AT+CGDCONT=1,\"IP\",\"test\"\r\n");
    harness.get_responses(); // Clear setup response
    harness.send_at_command(b"AT+CGQMIN=1,2,3,4,5,6\r\n");
    harness.get_responses(); // Clear setup response
    harness.send_at_command(b"AT+CGQMIN?\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CGQMIN: 1,2,3,4,5,6\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_set_and_query_quality_of_service_requested_gprs() {
    let harness = TestHarness::new();
    // First, define the context we are about to modify
    harness.send_at_command(b"AT+CGDCONT=1,\"IP\",\"test\"\r\n");
    harness.get_responses(); // Clear setup response
    harness.send_at_command(b"AT+CGQREQ=1,2,3,4,5,6\r\n");
    harness.get_responses(); // Clear setup response
    harness.send_at_command(b"AT+CGQREQ?\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CGQREQ: 1,2,3,4,5,6\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_set_ps_attach() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CGATT=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_pdp_context_modify() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CGCMOD=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_enter_data_state() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CGDATA=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"CONNECT\r\n");
}

#[test]
fn test_set_packet_event_reporting() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CGEREP=1,1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_show_pdp_address() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CGDCONT=1,\"IP\",\"test\"\r\n");
    harness.get_responses(); // Clear setup response
    harness.send_at_command(b"AT+CGPADDR=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CGPADDR: 1,\"0.0.0.0\"\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}
