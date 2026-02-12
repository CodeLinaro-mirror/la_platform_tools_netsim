// tests/network_service_test.rs

use std::time::Duration;

use crate::common::TestHarness;

#[test]
fn test_cops_query() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+COPS?\r\n");

    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+COPS: 0,0,\"Android Virtual Operator\"\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_csq_query() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CSQ\r\n");

    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CSQ: 20,99\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_network_registration() {
    let harness = TestHarness::new();

    // Advance the clock to trigger the registration event
    harness.clock.advance(Duration::from_millis(20));

    // Tick the simulator to process the event
    harness.manager.tick();

    // Verify that the modem sends a +CREG: 1 unsolicited response
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"+CREG: 1\r\n");
}

#[test]
fn test_query_extended_signal_quality() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CESQ\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}
