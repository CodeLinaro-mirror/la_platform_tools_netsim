mod common;
use common::TestHarness;

#[test]
fn test_set_ipr() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+IPR=9600\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_cmee() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CMEE=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_echo() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATE1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_speaker_volume() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATL1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_speaker_mute() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATM1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_quiet_mode() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATQ1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_verbose_mode() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATV1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_reset_to_factory_defaults() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT&F\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_view_active_configuration() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT&V\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_write_active_configuration() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT&W\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_reset() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATZ\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_get_identification_information() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATI\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_auto_answer() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATS0=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_command_termination_character() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATS3=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_response_formatting_character() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATS4=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_command_line_editing_character() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATS5=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_pause_before_blind_dialing() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATS6=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_connection_completion_timeout() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATS7=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_comma_dial_modifier_time() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATS8=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}

#[test]
fn test_set_automatic_disconnect_delay() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATS10=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"OK\r\n");
}
