use std::sync::Arc;

use modem_rs::{
    test_utils::{MockModemHandler, MockNetworkHandler},
    time::MockClock,
    *,
};

#[test]
fn test_add_modem_to_manager() {
    let manager_handler = Arc::new(MockNetworkHandler::new());
    let clock = Arc::new(MockClock::new());
    let manager = ModemNetworkSimulator::new_with_clock(manager_handler.clone(), clock);

    let modem_id: ModemId = 1;
    let modem_handler = Arc::new(MockModemHandler::new());
    manager.new_modem(modem_id, modem_handler.clone()).unwrap();

    assert_eq!(manager.get_modem_count(), 1);
}

#[test]
fn test_metrics_counters() {
    let manager_handler = Arc::new(MockNetworkHandler::new());
    let clock = Arc::new(MockClock::new());
    let manager = ModemNetworkSimulator::new_with_clock(manager_handler.clone(), clock);

    let modem_id: ModemId = 1;
    let modem_handler = Arc::new(MockModemHandler::new());
    manager.new_modem(modem_id, modem_handler.clone()).unwrap();

    // Check initial state
    let initial_metrics = manager.get_metrics();
    assert_eq!(initial_metrics.at_commands_received, 0);
    assert_eq!(initial_metrics.calls_initiated, 0);

    // Send a command and check again
    manager.send_at_command(modem_id, b"AT\r\n");
    let metrics_after_at = manager.get_metrics();
    assert_eq!(metrics_after_at.at_commands_received, 1);
    assert_eq!(metrics_after_at.calls_initiated, 0);

    // Initiate a call and check again
    manager.send_at_command(modem_id, b"ATD12345;\r\n");
    let metrics_after_dial = manager.get_metrics();
    assert_eq!(metrics_after_dial.at_commands_received, 2);
    assert_eq!(metrics_after_dial.calls_initiated, 1);
}
