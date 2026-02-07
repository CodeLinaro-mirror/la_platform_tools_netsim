// tests/integration_test.rs

use std::sync::Arc;

use modem_rs::{
    test_utils::{MockModemHandler, MockNetworkHandler},
    time::MockClock,
    types::{ModemId, AT_OK},
    ModemNetworkSimulator,
};

use crate::common::constants;

#[test]
fn test_two_modem_end_to_end_scenario() {
    // 1. Setup: Create a manager and two modems (A and B).
    let manager_handler = Arc::new(MockNetworkHandler::new());
    let clock = Arc::new(MockClock::new());
    let manager = ModemNetworkSimulator::new_with_clock(manager_handler.clone(), clock);

    let modem_a_id: ModemId = 1;
    let modem_a_handler = Arc::new(MockModemHandler::new());
    manager.new_modem(modem_a_id, modem_a_handler.clone()).unwrap();

    let modem_b_id: ModemId = 2;
    let modem_b_handler = Arc::new(MockModemHandler::new());
    manager.new_modem(modem_b_id, modem_b_handler.clone()).unwrap();

    // Give Modem B a phone number so Modem A can call it.
    manager.get_modem(modem_b_id).unwrap().set_phone_number(constants::PHONE_NUMBER_A);

    // 2. Power On & Check Network (Modem A)
    manager.send_at_command(modem_a_id, b"AT+CPIN?\r\n");
    assert_eq!(modem_a_handler.wait_for_response(), b"+CPIN: READY\r\n");
    assert_eq!(modem_a_handler.wait_for_response(), AT_OK);

    manager.send_at_command(modem_a_id, b"AT+COPS?\r\n");
    assert_eq!(modem_a_handler.wait_for_response(), b"+COPS: 0,0,\"Android Virtual Operator\"\r\n");
    assert_eq!(modem_a_handler.wait_for_response(), AT_OK);

    // 3. Make a Call
    // Modem A dials Modem B's number.
    manager
        .send_at_command(modem_a_id, format!("ATD{};\r\n", constants::PHONE_NUMBER_A).as_bytes());
    assert_eq!(modem_a_handler.wait_for_response(), AT_OK);

    // Verify Modem A is in the Dialing state.
    let modem_a = manager.get_modem(modem_a_id).unwrap();
    assert!(modem_a.call_service().is_dialing());

    // Verify that Modem B receives a RING notification and its state is Alerting.
    assert_eq!(modem_b_handler.wait_for_response(), b"RING\r\n");
    let modem_b = manager.get_modem(modem_b_id).unwrap();
    assert!(modem_b.call_service().is_alerting());

    // 4. Answer and Talk
    // Modem B answers the call.
    manager.send_at_command(modem_b_id, b"ATA\r\n");
    assert_eq!(modem_b_handler.wait_for_response(), b"OK\r\n");

    // Modem A should receive an OK to indicate the call is connected.
    assert_eq!(modem_a_handler.wait_for_response(), AT_OK);

    // Verify that both modems are now in the Active call state.
    assert!(modem_a.call_service().is_active());
    assert!(modem_b.call_service().is_active());

    // 5. Hang Up
    // Modem A hangs up the call.
    manager.send_at_command(modem_a_id, b"ATH\r\n");
    assert_eq!(modem_a_handler.wait_for_response(), AT_OK);

    // Verify that both modems are now Idle.
    assert!(modem_a.call_service().is_idle());
    assert!(modem_b.call_service().is_idle());

    // 6. Send SMS
    // Modem B sends an SMS to Modem A.
    // The PDU is a simplified representation for "hello".
    manager.send_at_command(modem_b_id, b"AT+CMGS=5\r\n");
    assert_eq!(modem_b_handler.wait_for_response(), b"> \r\n");
    manager.send_at_command(modem_b_id, b"hello\x1a");
    assert!(modem_b_handler.wait_for_response().starts_with(b"+CMGS: "));
    assert_eq!(modem_b_handler.wait_for_response(), b"OK\r\n");

    // Verify that Modem A received the SMS.
    assert_eq!(modem_a_handler.wait_for_response(), b"+CMT: ,5\r\nhello\r\n");
}
