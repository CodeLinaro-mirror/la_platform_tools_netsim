// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{sync::Arc, time::Duration};

use modem_rs::{
    ModemEvent, ModemId, ModemNetworkSimulator, test_utils::MockModemHandler, time::MockClock,
};

#[test]
fn test_event_loop_tick_and_duration() {
    let clock = Arc::new(MockClock::default());
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let mut simulator = ModemNetworkSimulator::new_with_clock(clock.clone(), tx);

    let modem_id: ModemId = 1;
    let (mut modem_handler, sink) = MockModemHandler::new();
    simulator.new_modem(modem_id, sink).unwrap();

    // Tick once to clear the initial registration event
    clock.advance(Duration::from_millis(10));
    simulator.tick();
    // Consume initial registration response(s)
    let _ = modem_handler.wait_for_response();
    while modem_handler.try_get_response().is_some() {}

    // 1. Schedule an event 100ms in the future.
    let event_duration = Duration::from_millis(100);
    simulator.schedule_event(modem_id, event_duration, ModemEvent::TestEvent);

    // 2. Tick before the event is due.
    // It should return the duration until the next event.
    let (events, next_duration) = simulator.tick();
    assert!(events.is_empty());
    assert_eq!(next_duration, Some(event_duration));

    // 3. Advance the clock manually.
    clock.advance(event_duration);

    // 4. Tick again. The event should fire now.
    // The queue should be empty, so it should return None (or Duration::ZERO/None
    // if empty).
    let (_events_after, next_duration_after) = simulator.tick();

    assert!(next_duration_after.is_none());

    // 5. Check that the event was handled.
    let response = modem_handler.wait_for_response();
    assert_eq!(response, b"TEST_EVENT_FIRED\r\n");
}
