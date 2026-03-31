// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{steps::*, world::World};

// Scenario: Add Modem to Manager
//   Given a modem "A"
//   Then modem count is 1
#[test]
fn test_add_modem_to_manager() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    then_modem_count_is(&mut world, 1);
}

// Scenario: Verify Metrics
//   Given a modem "A"
//   Then metrics are (AT=0, Calls=0)
//   When AT command "AT" is sent to "A"
//   Then metrics are (AT=1, Calls=0)
//   When AT command "ATD12345;" is sent to "A"
//   Then metrics are (AT=2, Calls=1)
#[test]
fn test_metrics_counters() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Check initial state
    then_metrics_are(&mut world, 0, 0);

    // Send a command and check again
    when_at_command_sent(&mut world, "A", "AT");
    then_metrics_are(&mut world, 1, 0);

    // Initiate a call and check again
    when_at_command_sent(&mut world, "A", "ATD12345;");
    then_metrics_are(&mut world, 2, 1);
}
