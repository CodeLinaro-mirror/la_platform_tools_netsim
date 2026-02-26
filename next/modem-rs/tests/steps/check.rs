use modem_rs::types::AT_OK;

use crate::world::World;

/// Check for expected modem count.
///
/// # Panics
///
/// Panics if the number of modems in `ModemNetworkSimulator` does not match
/// `expected`.
pub fn then_modem_count_is(world: &World, expected: usize) {
    assert_eq!(world.manager.get_modem_count(), expected);
}

/// Check internal metrics (AT commands, Calls).
///
/// Verifies that the manager's internal counters match the expected values.
pub fn then_metrics_are(world: &World, at_commands: u64, calls: u64) {
    let metrics = world.manager.get_metrics();
    assert_eq!(metrics.at_commands_received, at_commands, "AT commands count mismatch");
    assert_eq!(metrics.calls_initiated, calls, "Calls initiated count mismatch");
}

/// Verifies that the NEXT response from the modem exactly matches the expected
/// string.
///
/// This consumes exactly one response from the internal queue.
/// If `expected` does not end with `\r\n`, it is appended automatically before
/// comparison, UNLESS `expected` is exactly "OK".
///
/// Use this when testing synchronous command responses where order is
/// guaranteed.
pub fn then_response_is(world: &World, name: &str, expected: &str) {
    let (_, handler) = world.get_modem(name);
    let expected_bytes = normalize_expected_response(expected);

    let response = handler.wait_for_response();
    assert_eq!(
        response,
        expected_bytes,
        "Modem {} received unexpected response.\nExpected: {:?}\nActual: {:?}",
        name,
        String::from_utf8_lossy(&expected_bytes),
        String::from_utf8_lossy(&response)
    );
}

/// Verifies that the modem eventually receives a response containing the
/// expected substring.
///
/// This is useful for asynchronous notifications (URCs) or when multiple lines
/// of output might be interleaved.
///
/// This function consumes up to 10 responses looking for a match.
pub fn then_response_contains(world: &World, name: &str, expected: &str) {
    then_wait_for_response_containing(world, name, expected);
}

/// Helper function that consumes responses until a match is found.
///
/// Returns the matching response string on success.
/// Panics if no match is found after `MAX_RESPONSE_RETRIES`.
pub fn then_wait_for_response_containing(world: &World, name: &str, expected: &str) -> String {
    let (_, handler) = world.get_modem(name);
    const MAX_RESPONSE_RETRIES: usize = 10;

    for _ in 0..MAX_RESPONSE_RETRIES {
        let response = handler.wait_for_response();
        let response_str = String::from_utf8_lossy(&response).to_string();
        if response_str.contains(expected) {
            return response_str;
        }
    }

    panic!(
        "Modem {} did not receive expected substring after {} attempts.\nExpected to contain: {:?}",
        name, MAX_RESPONSE_RETRIES, expected
    );
}

/// Normalizes the expected response string to bytes.
/// Adds \r\n unless it's "OK" or already present.
fn normalize_expected_response(expected: &str) -> Vec<u8> {
    if expected == "OK" {
        AT_OK.to_vec()
    } else {
        let s = if expected.ends_with("\r\n") {
            expected.to_string()
        } else {
            format!("{}\r\n", expected)
        };
        s.as_bytes().to_vec()
    }
}
