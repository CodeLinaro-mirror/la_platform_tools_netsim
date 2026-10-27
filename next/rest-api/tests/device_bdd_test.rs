// Copyright 2026 The Android Open Source Project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Tier 2 Actor BDD integration tests for `/v1/devices` and `/v1/devices:reset`
//! over an ephemeral in-process HTTP listener (`netsim_rest_api::server::run`).

use netsim_model::{ChipVariant, DeviceId, MODEM_STATE_IDLE, Position};
use netsim_rest_api::ListDeviceResponse;

use crate::world::World;

// Feature: Device REST Endpoints and HTTP Body Parsing
//
//   As a netsim CLI or external HTTP client
//   I want GET /v1/devices and POST /v1/devices:reset served over HTTP/1.1
//   So that device state, pose, radio/cellular state, and reset work end-to-end

// Scenario: GET /v1/devices serializes pose.position and distinct Cell radio.state vs modem state
//   Given a running REST server with a Cellular device at (1.5, 2.5, 3.5)
//   When a client requests GET /v1/devices over HTTP
//   Then the server responds with 200 OK and Content-Type: application/json
//   And the raw JSON body contains pose.position (x, y, z) and distinct Cell.radio.state (bool) vs Cell.state (string)
//   And NetsimRestClient::get_devices() deserializes the response into ListDeviceResponse
#[tokio::test]
async fn test_bdd_get_devices_http_body_parsing_pose_and_cell_state() {
    // Given a running REST server with a Cellular device at (1.5, 2.5, 3.5)
    let world = World::new().await;
    let expected_pos = Position { x: 1.5, y: 2.5, z: 3.5 };
    let device_id = world.given_cellular_device("pixel-cell", expected_pos).await;
    assert_eq!(device_id, DeviceId(1));

    // When a client requests GET /v1/devices over HTTP
    let resp = world.when_http_request("GET", "/v1/devices", None, &[]).await;

    // Then the server responds with 200 OK and application/json headers
    assert_eq!(resp.status_code, 200);
    resp.assert_json_headers();

    // And the raw JSON body has pose.position and distinct Cell.radio.state (bool) vs Cell.state (string)
    let json = resp.parse_json_value();
    let dev_json = &json["devices"][0];
    assert_eq!(dev_json["id"], 1);
    assert_eq!(dev_json["name"], "pixel-cell");
    assert_eq!(dev_json["pose"]["position"]["x"], 1.5);
    assert_eq!(dev_json["pose"]["position"]["y"], 2.5);
    assert_eq!(dev_json["pose"]["position"]["z"], 3.5);

    let cell_json = &dev_json["chips"][0]["variant"]["Cell"];
    assert_eq!(cell_json["radio"]["state"], serde_json::Value::Bool(true));
    assert_eq!(cell_json["state"], serde_json::Value::String(MODEM_STATE_IDLE.to_string()));

    // And typed deserialization via RawHttpResponse and NetsimRestClient matches
    let parsed: ListDeviceResponse = resp.parse_json();
    assert_eq!(parsed.devices.len(), 1);
    assert_eq!(parsed.devices[0].pose.position, expected_pos);
    match &parsed.devices[0].chips[0].variant {
        Some(ChipVariant::Cell(cell)) => {
            assert_eq!(cell.radio.state, Some(true));
            assert_eq!(cell.state, MODEM_STATE_IDLE);
        }
        other => panic!("Expected ChipVariant::Cell, got {other:?}"),
    }

    let client_resp = world.rest_client.get_devices().send().await.unwrap();
    assert_eq!(client_resp, parsed);
}

// Scenario: POST /v1/devices:reset accepts both {} JSON body and empty body and resets actors
//   Given a running REST server with a dynamic Wi-Fi device and a dynamic AP
//   When a client sends POST /v1/devices:reset with {} JSON body
//   Then the server responds with 204 No Content and removes both the dynamic device and dynamic AP
//   When another dynamic Wi-Fi device and dynamic AP are created and reset with an empty body
//   Then the server again responds with 204 No Content and resets both actors
#[tokio::test]
async fn test_bdd_reset_devices_and_aps_empty_and_json_body() {
    let world = World::new().await;

    // Given a dynamic Wi-Fi device and a dynamic AP
    let dev1 = world.given_wifi_device("wifi-phone-1", Position::default()).await;
    let ap1 = world.given_dynamic_ap("TestWifi1").await;
    assert_eq!(dev1, DeviceId(1));
    assert!(ap1 > 0);
    assert_eq!(world.rest_client.get_devices().send().await.unwrap().devices.len(), 1);
    assert!(world.rest_client.get_aps().send().await.unwrap().aps.iter().any(|a| a.id == ap1));

    // When a client sends POST /v1/devices:reset with a "{}" JSON body
    let reset_json_resp = world
        .when_http_request("POST", "/v1/devices:reset", Some("application/json"), br#"{}"#)
        .await;

    // Then the server returns 204 No Content with empty body and resets both actors
    assert_eq!(reset_json_resp.status_code, 204);
    reset_json_resp.assert_no_content_headers();
    assert!(world.rest_client.get_devices().send().await.unwrap().devices.is_empty());
    let aps_after_first = world.rest_client.get_aps().send().await.unwrap().aps;
    assert!(!aps_after_first.iter().any(|a| a.id == ap1));
    assert!(aps_after_first.iter().any(|a| a.id == 0), "Default AP 0 must remain after reset");

    // Given a second dynamic Wi-Fi device and dynamic AP
    let _dev2 = world.given_wifi_device("wifi-phone-2", Position::default()).await;
    let ap2 = world.given_dynamic_ap("TestWifi2").await;
    assert_eq!(world.rest_client.get_devices().send().await.unwrap().devices.len(), 1);
    assert!(world.rest_client.get_aps().send().await.unwrap().aps.iter().any(|a| a.id == ap2));

    // When a client sends POST /v1/devices:reset with an empty body (raw HTTP and NetsimRestClient)
    let reset_empty_resp = world.when_http_request("POST", "/v1/devices:reset", None, &[]).await;
    assert_eq!(reset_empty_resp.status_code, 204);
    reset_empty_resp.assert_no_content_headers();
    assert!(world.rest_client.get_devices().send().await.unwrap().devices.is_empty());
    assert!(!world.rest_client.get_aps().send().await.unwrap().aps.iter().any(|a| a.id == ap2));

    world.rest_client.reset().send().await.expect("NetsimRestClient::reset() must succeed");
}

// Scenario: HTTP request body parsing rejects malformed JSON with 400 and oversized bodies with 413
//   Given a running REST server
//   When a client sends malformed JSON, empty body, or wrong field types to a JSON endpoint
//   Then the server responds with 400 Bad Request and ErrorResponse {"error": "Invalid JSON body: ..."}
//   When a client sends a request body exceeding 1 MiB
//   Then the server responds with 413 Payload Too Large and ErrorResponse {"error": "413 Payload Too Large"}
//   When a client requests an unknown route
//   Then the server responds with 404 Not Found and ErrorResponse {"error": "Not Found: ..."}
#[tokio::test]
async fn test_bdd_http_body_parsing_errors_and_size_limits() {
    let world = World::new().await;

    // 1. Malformed JSON syntax -> 400 Bad Request with ErrorResponse JSON
    let bad_json =
        world.when_http_request("POST", "/v1/aps", Some("application/json"), b"{bad json").await;
    assert_eq!(bad_json.status_code, 400);
    bad_json.assert_json_headers();
    assert!(
        bad_json.parse_error().error.starts_with("Invalid JSON body:"),
        "Unexpected error body: {:?}",
        bad_json.body
    );

    // 2. Empty body on endpoint requiring JSON -> 400 Bad Request
    let empty_body =
        world.when_http_request("POST", "/v1/aps", Some("application/json"), b"").await;
    assert_eq!(empty_body.status_code, 400);
    empty_body.assert_json_headers();
    assert!(empty_body.parse_error().error.starts_with("Invalid JSON body:"));

    // 3. Wrong field type in JSON body -> 400 Bad Request
    let wrong_type = world
        .when_http_request(
            "POST",
            "/v1/aps",
            Some("application/json"),
            br#"{"ssid":"Test","channel":"not-a-u8"}"#,
        )
        .await;
    assert_eq!(wrong_type.status_code, 400);
    wrong_type.assert_json_headers();
    assert!(wrong_type.parse_error().error.starts_with("Invalid JSON body:"));

    // 4. Oversized payload (> 1 MiB declared Content-Length) -> 413 Payload Too Large
    let oversized_len = (1 << 20) + 1;
    let oversized = world
        .when_http_request_with_content_length(
            "POST",
            "/v1/devices:reset",
            Some("application/json"),
            oversized_len,
            b"",
        )
        .await;
    assert_eq!(oversized.status_code, 413);
    oversized.assert_json_headers();
    assert_eq!(oversized.parse_error().error, "413 Payload Too Large");

    // 5. Unknown route -> 404 Not Found with ErrorResponse JSON
    let not_found = world.when_http_request("GET", "/v1/devices/unknown/route", None, &[]).await;
    assert_eq!(not_found.status_code, 404);
    not_found.assert_json_headers();
    assert_eq!(not_found.parse_error().error, "Not Found: GET /v1/devices/unknown/route");
}
