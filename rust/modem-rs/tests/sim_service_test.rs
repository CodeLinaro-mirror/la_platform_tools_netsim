mod common;
use common::TestHarness;

#[test]
fn test_cpin_query() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CPIN?\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CPIN: READY\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_cpin_set() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CPIN=\"1234\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_cimi() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CIMI\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"123456789012345\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_cicc() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CICCID\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"89012345678901234567\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

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

#[test]
fn test_open_logical_channel() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CCHO=\"1234\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert!(responses[0].starts_with(b"+CCHO: "));
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_close_logical_channel() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CCHO=\"1234\"\r\n");
    harness.get_responses(); // Clear responses
    harness.send_at_command(b"AT+CCHC=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");

    // Verify that the channel is closed by trying to transmit on it
    harness.send_at_command(b"AT+CGLA=1,10,\"00A40004022FE2\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"ERROR\r\n");
}

#[test]
fn test_transmit_logical_channel() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CCHO=\"1234\"\r\n");
    harness.get_responses(); // Clear responses
    harness.send_at_command(b"AT+CGLA=1,10,\"00A40004022FE2\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert!(responses[0].starts_with(b"+CGLA: "));
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_change_password() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CPWD=\"SC\",\"1234\",\"4321\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");

    // Verify that the old password doesn't work
    harness.send_at_command(b"AT+CPIN=\"1234\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"ERROR\r\n");

    // Verify that the new password works
    harness.send_at_command(b"AT+CPIN=\"4321\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_cdma_subscription_source() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CCSS=1\r\n");
    harness.get_responses(); // Clear responses
    harness.send_at_command(b"AT+CCSS?\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CCSS: 1\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_set_cdma_roaming_preference() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+WRMP=1\r\n");
    harness.get_responses(); // Clear responses
    harness.send_at_command(b"AT+WRMP?\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+WRMP: 1\r\n");
    assert_eq!(responses[1], b"OK\r\n");
}

#[test]
fn test_sim_authentication() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+MBAU=\"some_data\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_update_phone_number() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+REMOTEUPADATEPHONENUMBER=\"1234567890\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}
