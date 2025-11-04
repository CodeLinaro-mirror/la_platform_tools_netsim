mod common;
use common::TestHarness;

#[test]
fn test_read_iccid() {
    common::init_logger();
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CRSM=176,12258,0,0,10\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CRSM: 144,0,\"89012345678901234567\"\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_select_mf() {
    common::init_logger();
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CRSM=162,16128,0,0,0\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CRSM: 144,0,\"6210\"\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}
