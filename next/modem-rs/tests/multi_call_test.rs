use modem_rs::types::{ModemId, AT_OK};

use crate::common::TestHarness;

#[test]
fn test_multi_call_scenario() {
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

    // 4. Modem A swaps between calls.
    harness.send_at_command(b"AT+CHLD=2\r\n");
    let responses = harness.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], AT_OK);

    // 5. Verify that B is now active and C is held.
    let modem_a = harness.manager.get_modem(harness.modem_id).unwrap();
    let modem_b = harness.manager.get_modem(modem_b_id).unwrap();
    let modem_c = harness.manager.get_modem(modem_c_id).unwrap();
    assert!(modem_a.call_service().is_active());
    assert!(modem_b.call_service().is_active());
    assert!(modem_c.call_service().is_held());
}
