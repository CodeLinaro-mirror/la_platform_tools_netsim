use crate::common::TestHarness;

#[test]
fn test_set_and_query_time() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CCLK=\"25/08/02,12:30:00+00\"\r\n");
    harness.send_at_command(b"AT+CCLK?\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 3);
    assert_eq!(responses[1], b"+CCLK: \"25/08/02,12:30:00+00\"\r\n");
    assert_eq!(responses[2], b"OK\r\n");
}
