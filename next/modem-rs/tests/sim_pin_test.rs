mod common;
use common::TestHarness;

#[test]
fn test_pin_retry_counter() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CPIN=\"0000\"\r\n");
    harness.get_responses(); // Clear responses
    harness.send_at_command(b"AT+CPIN=\"0000\"\r\n");
    harness.get_responses(); // Clear responses
    harness.send_at_command(b"AT+SPIC\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+SPIC: 1\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}
