use crate::{steps::*, world::World};

// Scenario: Query Operator Selection
//   Given a modem "A"
//   When AT command "AT+COPS?" is sent to "A"
//   Then response from "A" is '+COPS: 0,0,"Android Virtual Operator"'
//   And response from "A" is "OK"
#[test]
fn test_cops_query() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+COPS?");

    then_response_is(&mut world, "A", "+COPS: 0,0,\"Android Virtual Operator\"");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Query Signal Quality
//   Given a modem "A"
//   When AT command "AT+CSQ" is sent to "A"
//   Then response from "A" is "+CSQ: 20,99"
//   And response from "A" is "OK"
#[test]
fn test_csq_query() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CSQ");

    then_response_is(&mut world, "A", "+CSQ: 20,99");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Network Registration
//   Given a modem "A"
//   When time advances 20 ms
//   Then response from "A" is "+CREG: 1"
#[test]
fn test_network_registration() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Advance the clock to trigger the registration event
    when_time_advances_ms(&mut world, 20);

    // Verify that the modem sends a +CREG: 1 unsolicited response
    then_response_is(&mut world, "A", "+CREG: 1");
}

// Scenario: Query Extended Signal Quality
//   Given a modem "A"
//   When AT command "AT+CESQ" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_query_extended_signal_quality() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CESQ");

    then_response_is(&mut world, "A", "OK");
}
