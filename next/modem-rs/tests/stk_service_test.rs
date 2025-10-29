use crate::common::TestHarness;

#[test]
fn test_stk_display_text() {
    let harness = TestHarness::new();
    // This is a simplified "Display Text" proactive command envelope.
    harness.send_at_command(b"AT+CUSATE=\"D1150121810D050448656C6C6F20576F726C64\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    // Expecting a terminal response acknowledging the command
    assert_eq!(responses[0], b"+CUSAT: \"9000\"\r\n");
}

#[test]
fn test_send_stk_envelope_command() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CUSATE=\"D3120101\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"+CUSATP: \"SubMenu1\"\r\n");
}

#[test]
fn test_stk_get_input() {
    let harness = TestHarness::new();
    // This is a simplified "Get Input" proactive command envelope.
    harness.send_at_command(b"AT+CUSATE=\"D1150123810D0504456E7465722054657874\"\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    // Expecting a terminal response acknowledging the command
    assert_eq!(responses[0], b"+CUSAT: \"9000\"\r\n");
}

#[test]
fn test_query_stk_ready() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CUSATD?\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"+CUSATP: \"SETUP MENU\"\r\n");
}

#[test]
fn test_set_stk() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+STK=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_stk_enabled() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+STKEN=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_stk_unsolicited_result() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+STKUR=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}
