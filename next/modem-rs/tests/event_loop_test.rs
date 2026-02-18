use std::{sync::Arc, time::Duration};

use modem_rs::{
    test_utils::MockNetworkHandler, time::MockClock, types::ModemId, ModemEvent,
    ModemNetworkSimulator,
};

#[test]
fn test_event_loop_tick_and_duration() {
    let clock = Arc::new(MockClock::new());
    let network_handler = Arc::new(MockNetworkHandler::new());
    let simulator = ModemNetworkSimulator::new_with_clock(network_handler, clock.clone());

    let modem_id: ModemId = 1;
    let modem_handler = Arc::new(crate::common::MockModemHandler::new());
    simulator.new_modem(modem_id, modem_handler.clone()).unwrap();

    // Tick once to clear the initial registration event
    clock.advance(Duration::from_millis(10));
    simulator.tick();
    modem_handler.get_responses();

    // 1. Schedule an event 100ms in the future.
    let event_duration = Duration::from_millis(100);
    simulator.schedule_event(modem_id, event_duration, ModemEvent::TestEvent);

    // 2. Tick before the event is due.
    // It should return the duration until the next event.
    let next_event_in = simulator.tick().unwrap();
    assert_eq!(next_event_in, event_duration);

    // 3. Advance the clock manually.
    clock.advance(event_duration);

    // 4. Tick again. The event should fire now.
    // The queue should be empty, so it should return None.
    let next_event_in_after = simulator.tick();
    assert!(next_event_in_after.is_none());

    // 5. Check that the event was handled.
    let responses = modem_handler.get_responses();
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], b"TEST_EVENT_FIRED\r\n");
}
