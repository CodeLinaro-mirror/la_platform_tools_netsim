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

    // Verify that the modem sends a +CREG: 1 and +CGREG: 1 unsolicited response
    then_response_is(&mut world, "A", "+CREG: 1");
    then_response_is(&mut world, "A", "+CGREG: 1");
}

// Scenario: Set dynamic registration status
//   Given a modem "A"
//   When voice registration is set to Roaming (5)
//   Then unsolicited response from "A" is "+CREG: 5"
//   When data registration is set to Denied (3)
//   Then unsolicited response from "A" is "+CGREG: 3"
#[test]
fn test_set_registration_status() {
    use modem_rs::RegistrationStatus;
    let mut world = World::new();
    given_modem(&mut world, "A");

    let id_a = world.modems.get("A").unwrap().0;

    // Set Voice
    when_voice_registration_set(&mut world, id_a, RegistrationStatus::Roaming);
    then_response_is(&mut world, "A", "+CREG: 5");

    // Set Data
    when_data_registration_set(&mut world, id_a, RegistrationStatus::Denied);
    then_response_is(&mut world, "A", "+CGREG: 3");
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

// Scenario: Set dynamic signal strength
//   Given a modem "A"
//   When signal strength is set to 25, 0
//   Then response from "A" to "AT+CSQ" is "+CSQ: 25,0"
#[test]
fn test_set_signal_strength() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Check default
    when_at_command_sent(&mut world, "A", "AT+CSQ");
    then_response_is(&mut world, "A", "+CSQ: 20,99");
    then_response_is(&mut world, "A", "OK");

    // Change value
    let id_a = world.modems.get("A").unwrap().0;
    when_signal_strength_set(&mut world, id_a, 25, 0);

    // Check new value
    when_at_command_sent(&mut world, "A", "AT+CSQ");
    then_response_is(&mut world, "A", "+CSQ: 25,0");
    then_response_is(&mut world, "A", "OK");
}
