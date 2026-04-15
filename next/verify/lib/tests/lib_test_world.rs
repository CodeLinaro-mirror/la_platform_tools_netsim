// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use features::{assert_json_matches_table, table_to_struct, DataTable, Features};
use netsim_packets::UdpHeader;
use verify_macros::{step, step_module};
use zerocopy::IntoBytes;

/// # Example World
///
/// This is an example of a "World" struct that holds the state for Intentions
/// tests. It demonstrates how to use the `features_codegen` macro to
/// automatically generate glue code.
///
/// The `features_codegen` tool (invoked via `genrule` in BUILD) parses this
/// file, finds methods commented with `/// STEP:`, and generates
/// `features_world_glue.rs`.
struct TestWorld {
    /// The main display register (accumulator)
    display: i32,
    log: Vec<String>,
    users: std::collections::HashMap<String, i32>,
    tx_power: TxPower,
    enabled: bool,
    temperature: f64,
    /// The memory register (M+, M-, MR, MC)
    memory: i32,
    /// Mock network interface buffer (stores received packets)
    received_packets: Vec<Vec<u8>>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum TxPower {
    High,
    Medium,
    Low,
    Ultralow,
}

impl std::str::FromStr for TxPower {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "High" => Ok(TxPower::High),
            "Medium" => Ok(TxPower::Medium),
            "Low" => Ok(TxPower::Low),
            "Ultralow" => Ok(TxPower::Ultralow),
            _ => Err(format!("Invalid TxPower: {}", s)),
        }
    }
}

impl features::World for TestWorld {}

impl TestWorld {}

#[step_module]
pub mod steps {
    use anyhow::Result;

    use super::*;

    #[step(r"I reset the counter")]
    async fn given_i_reset_the_counter(w: &mut TestWorld) {
        w.display = 0;
        w.log.push("reset".to_string());
    }

    #[step(r"I have a calculator with Memory (\d+)")]
    async fn given_i_have_a_calculator_with_memory(w: &mut TestWorld, memory: i32) {
        w.memory = memory;
        w.log.push(format!("reset_calculator memory {}", memory));
    }

    #[step(r"I have a calculator with No Memory")]
    async fn given_i_have_a_calculator_with_no_memory(w: &mut TestWorld) {
        w.memory = 0;
        w.log.push("reset_calculator memory 0".to_string());
    }

    #[step(r"I want to ensure a clean state before every scenario")]
    async fn given_clean_state(w: &mut TestWorld) {
        w.display = 0;
        w.log.push("reset_background".to_string());
    }

    #[step(r"I organize my scenarios with tags")]
    async fn given_tags(w: &mut TestWorld) {
        w.log.push("reset_tags".to_string());
    }

    #[step(r"I set TxPower to (High|Medium|Low|Ultralow)")]
    async fn when_i_set_tx_power(w: &mut TestWorld, level: TxPower) {
        w.tx_power = level;
        w.log.push(format!("tx_power {:?}", level));
    }

    #[step(r"I add (\d+)")]
    async fn when_i_add(w: &mut TestWorld, val: i32) {
        w.display += val;
        w.log.push(format!("add {}", val));
    }

    #[step(r"result is (\d+)")]
    async fn then_result_is(w: &mut TestWorld, val: i32) {
        assert_eq!(w.display, val, "Check failed: expected {}, got {}", val, w.display);
        w.log.push(format!("check {}", val));
    }

    #[step(r"system is (true|false)")]
    async fn given_system_is(w: &mut TestWorld, state: bool) {
        w.enabled = state;
        w.log.push(format!("system is {}", state));
    }

    #[step(r"I set temperature to (\d+\.\d+)")]
    async fn when_i_set_temperature(w: &mut TestWorld, temp: f64) {
        w.temperature = temp;
        w.log.push(format!("temperature set to {}", temp));
    }

    #[step(r"system should be (true|false)")]
    async fn then_system_should_be(w: &mut TestWorld, state: bool) {
        assert_eq!(w.enabled, state);
        w.log.push(format!("check system is {}", state));
    }

    #[step(r"temperature should be (\d+\.\d+)")]
    async fn then_temperature_should_be(w: &mut TestWorld, temp: f64) {
        assert!((w.temperature - temp).abs() < 0.001);
        w.log.push(format!("check temperature is {}", temp));
    }

    pub async fn before_hook(w: &mut TestWorld) {
        w.display = 0;
        w.log.push("before".to_string());
    }

    pub async fn after_hook(w: &mut TestWorld) {
        w.log.push("after".to_string());
    }

    #[step(r"the following users:")]
    async fn given_users(w: &mut TestWorld, table: DataTable) {
        for row in table.iter().skip(1) {
            let name = row[0].clone();
            let age: i32 = row[1].parse().expect("Age must be a number");
            w.users.insert(name.clone(), age);
            w.log.push(format!("user {} age {}", name, age));
        }
    }

    #[step(r"lookup (\w+) is (\d+)")]
    async fn then_lookup(w: &mut TestWorld, name: String, age: i32) {
        let actual = w.users.get(&name).expect("User not found");
        assert_eq!(*actual, age, "Age mismatch for {}", name);
        w.log.push(format!("lookup {} is {}", name, age));
    }

    #[step(r"I have a UDP Echo Server on Port (\d+)")]
    async fn given_udp_echo_server(w: &mut TestWorld, _port: u16) {
        w.received_packets.clear();
        w.log.push("udp_echo_server_init".to_string());
    }

    #[step(r"I send a UDP packet with:")]
    async fn when_send_packet(w: &mut TestWorld, table: DataTable) {
        let json_header: netsim_packets::JsonUdpHeader =
            table_to_struct(&table).expect("Failed to parse UdpHeader");
        let header: UdpHeader =
            json_header.try_into().expect("Failed to convert JsonUdpHeader to UdpHeader");
        w.received_packets.push(header.as_bytes().to_vec());
        w.log.push("send_udp".to_string());
    }

    #[step(r"I receive a UDP packet matching:")]
    async fn then_receive_packet(w: &mut TestWorld, table: DataTable) {
        assert!(!w.received_packets.is_empty(), "No packets received");
        let last_packet = w.received_packets.last().unwrap();
        let (header, _payload) =
            UdpHeader::parse(last_packet.as_slice()).expect("Failed to parse UDP header");
        let packet_json = netsim_packets::to_json(&header);
        assert_json_matches_table(&packet_json, &table);
        w.log.push("check_udp".to_string());
    }

    #[step(r"I setup with default users:")]
    async fn given_setup_default_users(w: &mut TestWorld, table: DataTable) {
        given_users(w, table).await;
    }

    #[step(r"I check if additive table works:")]
    async fn check_additive_table(w: &mut TestWorld, table: DataTable) {
        use features::horizontal_table_to_structs;
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct KeyVal {
            key: String,
            value: String,
        }

        let rows: Vec<KeyVal> = horizontal_table_to_structs(&table).expect("Failed to parse table");

        for row in rows {
            if row.key == "add" {
                let val: i32 = row.value.parse().expect("Value must be a number");
                w.display += val;
                w.log.push(format!("add {}", val));
            }
        }
    }

    #[step(r"I fail with an error")]
    async fn when_i_fail_with_an_error(_w: &mut TestWorld) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("Intentional failure"))
    }
}

fn before_helper<'a>(
    w: &'a mut TestWorld,
    _args: Vec<String>,
    _ctx: features::StepContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + 'a>> {
    Box::pin(async move {
        steps::before_hook(w).await;
        Ok(())
    })
}

fn after_helper<'a>(
    w: &'a mut TestWorld,
    _args: Vec<String>,
    _ctx: features::StepContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + 'a>> {
    Box::pin(async move {
        steps::after_hook(w).await;
        Ok(())
    })
}

fn setup_features_world() -> (Features<TestWorld>, TestWorld) {
    let mut features = Features::<TestWorld>::new();
    steps::register_steps(&mut features);
    features.before(before_helper);
    features.after(after_helper);
    let world = TestWorld {
        display: 0,
        log: Vec::new(),
        users: std::collections::HashMap::new(),
        tx_power: TxPower::Low,
        enabled: false,
        temperature: 0.0,
        memory: 0,
        received_packets: Vec::new(),
    };
    (features, world)
}

#[tokio::test]
async fn test_features_methods_success() {
    let (features, mut world) = setup_features_world();
    world.display = 999;

    features.run("I reset the counter", &mut world).await.unwrap();
    features.run("I add 10", &mut world).await.unwrap();
    features.run("result is 10", &mut world).await.unwrap();

    assert_eq!(world.log, vec!["reset", "add 10", "check 10"]);
}

#[tokio::test]
async fn test_features_background() {
    netsim_testing::logger::setup(None);
    let (features, mut world) = setup_features_world();

    features
        .execute_from_memory(include_str!("features/background.feat"), &mut world)
        .await
        .unwrap();

    // Feature: Background Support
    // Background: reset_background
    // Scenario: Incremental Add (add 5, check 5)
    //
    // Sequence:
    // Before -> Background(reset_background) -> Steps(add 5, check 5) -> After

    assert_eq!(world.log, vec!["before", "reset_background", "add 5", "check 5", "after"]);
}

#[tokio::test]
async fn test_features_outline() {
    netsim_testing::logger::setup(None);
    let (features, mut world) = setup_features_world();

    features.execute_from_memory(include_str!("features/outline.feat"), &mut world).await.unwrap();

    // Feature: Scenario Outline Support
    // Background: reset_outline
    // Scenario Outline: Add multiple numbers
    //   Ex 1: 10, 20 -> 30
    //   Ex 2: 1, 2 -> 3
    //
    // Sequence:
    // Before -> Background(reset_outline) -> Steps(add 10, add 20, check 30) ->
    // After Before -> Background(reset_outline) -> Steps(add 1, add 2, check 3)
    // -> After

    assert_eq!(
        world.log,
        vec![
            "before",
            "reset_calculator memory 0",
            "add 10",
            "add 20",
            "check 30",
            "after",
            "before",
            "reset_calculator memory 0",
            "add 1",
            "add 2",
            "check 3",
            "after"
        ]
    );
}

#[tokio::test]
#[should_panic(expected = "Check failed: expected 9999, got 10")]
async fn test_features_assertion_failure() {
    let (features, mut world) = setup_features_world();

    features.run("I reset the counter", &mut world).await.unwrap();
    features.run("I add 10", &mut world).await.unwrap();
    // This should panic
    features.run("result is 9999", &mut world).await.unwrap();
}

#[tokio::test]
async fn test_features_missing_step() {
    let (features, mut world) = setup_features_world();
    let err = features.run("I do not exist", &mut world).await.unwrap_err();
    assert_eq!(err.to_string(), "No step definition found for: I do not exist");
}

#[tokio::test]
async fn test_features_tags_filtering() {
    netsim_testing::logger::setup(None);
    let (mut features, mut world) = setup_features_world();

    // Set filter to @wip
    features.filter("@wip");

    features.execute_from_memory(include_str!("features/tags.feat"), &mut world).await.unwrap();

    // Filtered execution (only Tagged scenario)
    // Before -> Background(reset_tags) -> Steps(add 100, check 100) -> After

    assert_eq!(world.log, vec!["before", "reset_tags", "add 100", "check 100", "after"]);
}

#[tokio::test]
async fn test_features_data_table() {
    netsim_testing::logger::setup(None);
    let (features, mut world) = setup_features_world();

    let feature = include_str!("features/data_table.feat");

    features.execute_from_memory(feature, &mut world).await.unwrap();

    assert_eq!(
        world.log,
        vec![
            "before",
            "user Alice age 30",
            "user Bob age 25",
            "lookup Alice is 30",
            "lookup Bob is 25",
            "after"
        ]
    );
}

#[tokio::test]
async fn test_features_types() {
    netsim_testing::logger::setup(None);
    let (features, mut world) = setup_features_world();

    let feature = include_str!("features/types.feat");

    features.execute_from_memory(feature, &mut world).await.unwrap();

    // Feature: Type Support
    // Scenario: Boolean and Float types
    //   Given system is true
    //   When I set temperature to 98.6
    //   Then system should be true
    //   And temperature should be 98.6
    //
    // Scenario: Disable system
    //   Given system is false
    //   Then system should be false

    assert_eq!(
        world.log,
        vec![
            "before",
            "system is true",
            "temperature set to 98.6",
            "check system is true",
            "check temperature is 98.6",
            "after",
            "before",
            "system is false",
            "check system is false",
            "after"
        ]
    );
}

#[tokio::test]
async fn test_features_enum() {
    netsim_testing::logger::setup(None);
    let (mut features, mut world) = setup_features_world();

    let feature_content = include_str!("features/enum.feat");

    features.execute_from_memory(feature_content, &mut world).await.unwrap();

    assert_eq!(world.log, vec!["before", "tx_power High", "tx_power Ultralow", "after"]);
    assert_eq!(world.tx_power, TxPower::Ultralow);
}

#[tokio::test]
async fn test_features_network() {
    netsim_testing::logger::setup(None);
    let (features, mut world) = setup_features_world();
    features.execute_from_memory(include_str!("features/network.feat"), &mut world).await.unwrap();

    assert_eq!(
        world.log,
        vec!["before", "udp_echo_server_init", "send_udp", "check_udp", "after",]
    );
}

#[tokio::test]
async fn test_features_background_table() {
    netsim_testing::logger::setup(None);
    let (features, mut world) = setup_features_world();

    features
        .execute_from_memory(include_str!("features/background_table.feature"), &mut world)
        .await
        .unwrap();

    assert_eq!(
        world.log,
        vec![
            "before",
            "user Admin age 99",
            "user Guest age 10",
            "lookup Admin is 99",
            "lookup Guest is 10",
            "after"
        ]
    );
}

#[tokio::test]
async fn test_features_methods_success_file() {
    netsim_testing::logger::setup(None);
    let (features, mut world) = setup_features_world();

    features
        .execute_from_memory(include_str!("features/methods_success.feature"), &mut world)
        .await
        .unwrap();

    assert_eq!(world.log, vec!["before", "reset", "add 10", "check 10", "after"]);
}

#[tokio::test]
async fn test_features_outline_table() {
    netsim_testing::logger::setup(None);
    let (features, mut world) = setup_features_world();

    features
        .execute_from_memory(include_str!("features/outline_table.feature"), &mut world)
        .await
        .unwrap();

    assert_eq!(
        world.log,
        vec![
            // First example: 10, 20 -> 30
            "before", "reset", "add 10", "add 20", // via table
            "check 30", "after", // Second example: 5, 5 -> 10
            "before", "reset", "add 5", "add 5", // via table
            "check 10", "after"
        ]
    );
}

#[tokio::test]
async fn test_features_result_error_propagation() {
    let (features, mut world) = setup_features_world();
    let result = features.run("I fail with an error", &mut world).await;
    assert!(result.is_err());
}
