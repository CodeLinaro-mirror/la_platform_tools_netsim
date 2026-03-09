use std::time::Duration;

use modem_rs::{
    constants::CALL_RING_TIMEOUT,
    types::{ModemId, AT_OK},
};

use crate::common::TestHarness;

#[test]
fn test_emergency_call() {
    let harness = TestHarness::new();
    harness.send_at_command(b"ATD911;\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], AT_OK);
}

#[test]
fn test_standard_call() {
    let harness = TestHarness::new();
    let number = "1234567";
    harness.send_at_command(format!("ATD{};\r\n", number).as_bytes());
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], AT_OK);
}

#[test]
fn test_ring() {
    let harness = TestHarness::new();
    let modem = harness.manager.get_modem(harness.modem_id).unwrap();
    modem.receive_at_command(b"RING\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"RING\r\n");
    let calls = modem.call_service.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].number, "");
    assert_eq!(calls[0].state, modem_rs::call_service::CallState::Alerting);
}

#[test]
fn test_query_current_calls() {
    // 1. Setup: Create a manager and three modems (A, B, and C).
    let harness = TestHarness::new();
    let modem_b_id: ModemId = 2;
    let modem_c_id: ModemId = 3;
    harness.manager.get_modem(modem_b_id).unwrap().set_phone_number("111");
    harness.manager.get_modem(modem_c_id).unwrap().set_phone_number("222");

    // 2. Modem A calls B, B answers.
    harness.send_at_command(b"ATD111;\r\n");
    harness.get_responses(); // Clear responses
    harness.manager.send_at_command(modem_b_id, b"ATA\r\n");
    harness.get_responses(); // Clear responses

    // 3. Modem A calls C, putting B on hold.
    harness.send_at_command(b"ATD222;\r\n");
    harness.get_responses(); // Clear responses
    harness.manager.send_at_command(modem_c_id, b"ATA\r\n");
    harness.get_responses(); // Clear responses

    // 4. Modem A queries current calls.
    harness.send_at_command(b"AT+CLCC\r\n");

    // 5. Verify the response.
    let mut responses = harness.get_responses();
    assert_eq!(responses.len(), 3);
    responses.sort();

    let expected1 = b"+CLCC: 1,0,1,0,0,\"111\",129\r\n"; // Held call
    let expected2 = b"+CLCC: 2,0,0,0,0,\"222\",129\r\n"; // Active call
    let expected_ok = AT_OK;

    assert!(responses.contains(&expected1.to_vec()));
    assert!(responses.contains(&expected2.to_vec()));
    assert!(responses.contains(&expected_ok.to_vec()));
}

#[test]
fn test_call_ring_timeout() {
    let harness = TestHarness::new();
    let modem_b_id: ModemId = 2;
    let modem_c_id: ModemId = 3;
    let modem_b = harness.manager.get_modem(modem_b_id).unwrap();
    modem_b.set_phone_number("111");
    let modem_c = harness.manager.get_modem(modem_c_id).unwrap();
    modem_c.set_phone_number("222");

    // Make a call from A to B, and have B answer
    harness.send_at_command(b"ATD111;\r\n");
    harness.manager.tick();
    harness.manager.send_at_command(modem_b_id, b"ATA\r\n");
    harness.manager.tick();

    // Make a call from A to C
    harness.send_at_command(b"ATD222;\r\n");
    harness.manager.tick();

    // Verify that modem C is ringing and B is on hold
    assert!(modem_c.call_service().is_alerting());
    assert!(modem_b.call_service().is_held());

    // Advance the clock to trigger the timeout
    harness.clock.advance(CALL_RING_TIMEOUT + Duration::from_millis(100));

    // Tick the simulator to process the timeout
    harness.manager.tick();

    // Verify that modem C is no longer ringing, but B is still on hold
    assert!(!modem_c.call_service().is_alerting());
    assert!(modem_c.call_service().is_idle());
    assert!(modem_b.call_service().is_held());
}

#[test]
fn test_set_mute() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CMUT=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], AT_OK);
}

#[test]
fn test_query_mute() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+CMUT=1\r\n");
    harness.get_responses(); // Clear responses
    harness.send_at_command(b"AT+CMUT?\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+CMUT: 1\r\n");
    assert_eq!(responses[1], AT_OK);
}

#[test]
fn test_send_dtmf() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+VTS=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], AT_OK);
}

#[test]
fn test_set_emergency_mode() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+WSOS=1\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], AT_OK);
}

#[test]
fn test_query_emergency_mode() {
    let harness = TestHarness::new();
    harness.send_at_command(b"AT+WSOS=1\r\n");
    harness.get_responses(); // Clear responses
    harness.send_at_command(b"AT+WSOS?\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], b"+WSOS: 1\r\n");
    assert_eq!(responses[1], AT_OK);
}
