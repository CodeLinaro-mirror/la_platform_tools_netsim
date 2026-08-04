// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

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
pub fn then_response_is(world: &mut World, name: &str, expected: &str) {
    let id = world.modems.get(name).map(|(id, _)| *id).expect("Modem not found");
    let goldfish_37 =
        world.manager.get_modem(id).is_some_and(|m| m.quirks.goldfish_ril_37_or_earlier);
    let expected_bytes = normalize_expected_response(expected, goldfish_37);

    let (_, handler) = world.get_modem(name);
    let response = handler.wait_for_response();
    assert_eq!(
        response,
        expected_bytes,
        "Modem {} received unexpected response.\nExpected: {:?}\nActual: {:?}",
        name,
        String::from_utf8_lossy(&expected_bytes),
        String::from_utf8_lossy(&response)
    );

    if is_final_result_code(expected) {
        assert_no_trailing_response(handler, name);
    }
}

/// Verifies that the modem eventually receives a response containing the
/// expected substring.
///
/// This is useful for asynchronous notifications (URCs) or when multiple lines
/// of output might be interleaved.
///
/// This function consumes up to 10 responses looking for a match.
pub fn then_response_contains(world: &mut World, name: &str, expected: &str) {
    then_wait_for_response_containing(world, name, expected);
}

/// Helper function that consumes responses until a match is found.
///
/// Returns the matching response string on success.
/// Panics if no match is found after `MAX_RESPONSE_RETRIES`.
pub fn then_wait_for_response_containing(world: &mut World, name: &str, expected: &str) -> String {
    let (_, handler) = world.get_modem(name);
    const MAX_RESPONSE_RETRIES: usize = 10;

    for _ in 0..MAX_RESPONSE_RETRIES {
        let response = handler.wait_for_response();
        let response_str = String::from_utf8_lossy(&response).to_string();
        if response_str.contains(expected) {
            if is_final_result_code(expected) {
                assert_no_trailing_response(handler, name);
            }
            return response_str;
        }
    }

    panic!(
        "Modem {name} did not receive expected substring after {MAX_RESPONSE_RETRIES} attempts.\nExpected to contain: {expected:?}"
    );
}

fn assert_no_trailing_response(handler: &mut modem_rs::test_utils::MockModemHandler, name: &str) {
    handler.buffer_pending_responses();
    let mut prev_was_sms_urc_header = false;
    for trailing in handler.get_buffer() {
        if is_unsolicited_response(trailing) {
            let s = String::from_utf8_lossy(trailing);
            prev_was_sms_urc_header = s.starts_with("+CMT:") || s.starts_with("+CDS:");
            continue;
        }
        if prev_was_sms_urc_header {
            prev_was_sms_urc_header = false;
            continue;
        }
        panic!(
            "Modem {} received unexpected trailing response:\n{:?}",
            name,
            String::from_utf8_lossy(trailing)
        );
    }
}

fn is_final_result_code(expected: &str) -> bool {
    expected == "OK"
        || expected.ends_with("OK")
        || expected == "ERROR"
        || expected.contains("ERROR")
        || expected == "NO CARRIER"
        || expected == "CONNECT"
}

fn is_unsolicited_response(response: &[u8]) -> bool {
    let s = String::from_utf8_lossy(response);
    s.starts_with("+CMT:")
        || s.starts_with("+CMTI:")
        || s.starts_with("+CDS:")
        || s.starts_with("+CUSATP:")
        || s.starts_with("RING")
        || s.starts_with("+CLIP:")
        || s.starts_with("+CREG:")
        || s.starts_with("+CGREG:")
        || s.starts_with("+CEREG:")
        || s.starts_with("+CGEV:")
        || s.starts_with("+CSQ:")
}

/// Normalizes the expected response string to bytes.
/// Adds \r\n (or \r for Goldfish 37) unless it's "OK" or already present.
fn normalize_expected_response(expected: &str, goldfish_37: bool) -> Vec<u8> {
    let suffix: &[u8] = if goldfish_37 { b"\r" } else { b"\r\n" };
    match expected {
        "OK" => [b"OK", suffix].concat(),
        _ => [expected.trim_end_matches(['\r', '\n']).as_bytes(), suffix].concat(),
    }
}

/// Verifies that the modem has no pending responses in its queue.
///
/// This is useful for asserting that an asynchronous notification (like +CMT)
/// was NOT received (e.g., when a message is dropped due to routing failure).
pub fn then_no_response(world: &mut World, name: &str) {
    let (_, handler) = world.get_modem(name);
    let response = handler.try_get_response();
    assert!(
        response.is_none(),
        "Expected no response from modem {}, but got: {:?}",
        name,
        String::from_utf8_lossy(&response.unwrap())
    );
}
