use crate::{steps::*, world::World};

// Scenario: Set and Query Time
//   Given a modem "A"
//   When AT command 'AT+CCLK="25/08/02,12:30:00+00"' is sent to "A"
//   Then response from "A" is "OK"
//   When AT command "AT+CCLK?" is sent to "A"
//   Then response from "A" is '+CCLK: "25/08/02,12:30:00+00"'
//   And response from "A" is "OK"
#[test]
fn test_set_and_query_time() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    let time_str = "\"25/08/02,12:30:00+00\"";
    let set_cmd = format!("AT+CCLK={}", time_str);

    when_at_command_sent(&mut world, "A", &set_cmd);
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CCLK?");
    then_response_is(&mut world, "A", &format!("+CCLK: {}", time_str));
    then_response_is(&mut world, "A", "OK");
}
