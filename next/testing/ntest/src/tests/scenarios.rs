//! # Netsim BDD Scenarios
//!
//! This module defines the sequence of operations for the integration test
//! suite.
//!
//! ## Variable Definition Pattern
//! To avoid "Ghost Variable" errors in dry-run mode, we follow a strict
//! definition pattern:
//! 1. **Definition**: Use literal names in quotes (e.g., `on "port"`) during
//!    the step that populates the variable.
//! 2. **Usage**: Use curly braces (e.g., `to 10.0.2.2:{port}`) in subsequent
//!    steps that consume the resolved value.

use anyhow::Result;

use super::{executor::ClientParams, TestContext};

/// Programmatic entry point for the entire integration test suite.
pub async fn run_suite(ctx: &mut TestContext) -> Result<()> {
    scenario_tcp_echo(ctx).await?;
    ctx.reset_agent().await?;

    scenario_udp_echo(ctx).await?;
    ctx.reset_agent().await?;

    scenario_wifi_service_discovery(ctx).await?;
    ctx.reset_agent().await?;

    scenario_gateway_performance(ctx).await?;
    ctx.reset_agent().await?;

    scenario_multi_step_coordination(ctx).await?;
    scenario_multi_avd_echo(ctx).await?;

    Ok(())
}

async fn scenario_wifi_service_discovery(ctx: &mut TestContext) -> Result<()> {
    println!("\nSCENARIO: WiFi Service Discovery (mDNS/NSD)");
    ctx.discover_devices(2).await?;

    ctx.log_step("AVD#1", "WHEN", "Android Advertises Service _http._tcp. as MyWiFiService");
    ctx.execute_step("AVD#1", "When Android Advertises Service _http._tcp. as MyWiFiService")
        .await?;

    ctx.log_step("AVD#2", "WHEN", "Android Starts Discovery for _http._tcp.");
    ctx.execute_step("AVD#2", "When Android Starts Discovery for _http._tcp.").await?;

    ctx.log_step("AVD#2", "THEN", "Android Finds Service MyWiFiService");
    ctx.execute_step("AVD#2", "Then Android Finds Service MyWiFiService").await?;

    Ok(())
}

async fn scenario_tcp_echo(ctx: &mut TestContext) -> Result<()> {
    println!("\nSCENARIO: Basic TCP Echo");
    ctx.discover_devices(1).await?;

    ctx.log_step("Host", "WHEN", "Starts a TCP echo server on \"port\"");
    let _port = ctx.start_server(0).await?;

    ctx.log_step("AVD#1", "AND", "Sends 1KB TCP to 10.0.2.2:{port}");
    ctx.run_client_check(
        "AVD#1",
        ClientParams {
            proto: "tcp".to_string(),
            target: "10.0.2.2:{port}".to_string(),
            payload_size: 1024,
            expect_eof: false,
            timeout_ms: 1000,
        },
    )
    .await?;

    ctx.log_step("Host", "THEN", "Receives 1KB TCP data");
    Ok(())
}

async fn scenario_udp_echo(ctx: &mut TestContext) -> Result<()> {
    println!("\nSCENARIO: Basic UDP Echo");
    ctx.discover_devices(1).await?;

    ctx.log_step("Host", "WHEN", "Starts a UDP echo server on \"port\"");
    let _port = ctx.start_server(0).await?;

    ctx.log_step("AVD#1", "AND", "Sends 512B UDP to 10.0.2.2:{port}");
    ctx.run_client_check(
        "AVD#1",
        ClientParams {
            proto: "udp".to_string(),
            target: "10.0.2.2:{port}".to_string(),
            payload_size: 512,
            expect_eof: false,
            timeout_ms: 1000,
        },
    )
    .await?;

    ctx.log_step("Host", "THEN", "Receives 512B UDP data");
    Ok(())
}

async fn scenario_gateway_performance(ctx: &mut TestContext) -> Result<()> {
    println!("\nSCENARIO: Gateway Performance");
    ctx.discover_devices(1).await?;

    ctx.log_step("Host", "WHEN", "Starts an echo server on the gateway on \"port\"");
    let _port = ctx.start_server(0).await?;

    ctx.log_step(
        "AVD#1",
        "THEN",
        "Measures performance with 10 samples of 1MB TCP to {gateway_ip}:{port}",
    );
    ctx.run_benchmark(
        "AVD#1",
        ClientParams {
            proto: "tcp".to_string(),
            target: "{gateway_ip}:{port}".to_string(),
            payload_size: 1024 * 1024,
            expect_eof: false,
            timeout_ms: 60000,
        },
        10,
    )
    .await?;

    ctx.log_step("Host", "THEN", "Receives 10MB TCP data total");
    Ok(())
}

async fn scenario_multi_step_coordination(ctx: &mut TestContext) -> Result<()> {
    println!("\nSCENARIO: Multi-step Coordination");
    ctx.discover_devices(1).await?;

    ctx.log_step("Host", "GIVEN", "An active echo server on \"port\"");
    let _port = ctx.start_server(0).await?;

    ctx.log_step("AVD#1", "WHEN", "Sends 2KB TCP to 10.0.2.2:{port}");
    ctx.run_client_check(
        "AVD#1",
        ClientParams {
            proto: "tcp".to_string(),
            target: "10.0.2.2:{port}".to_string(),
            payload_size: 2048,
            expect_eof: false,
            timeout_ms: 5000,
        },
    )
    .await?;

    ctx.log_step("AVD#1", "AND", "Sends 512B UDP to 10.0.2.2:{port}");
    ctx.run_client_check(
        "AVD#1",
        ClientParams {
            proto: "udp".to_string(),
            target: "10.0.2.2:{port}".to_string(),
            payload_size: 512,
            expect_eof: false,
            timeout_ms: 2000,
        },
    )
    .await?;

    ctx.log_step("Host", "THEN", "Receives all coordinated data");
    Ok(())
}

async fn scenario_multi_avd_echo(ctx: &mut TestContext) -> Result<()> {
    println!("\nSCENARIO: Multi-AVD Echo");

    ctx.discover_devices(2).await?;

    ctx.log_step("Host", "WHEN", "Starts a TCP echo server on \"port\"");
    let _port = ctx.start_server(0).await?;

    ctx.log_step("AVD#1", "AND", "Sends 1KB TCP to 10.0.2.2:{port}");
    ctx.run_client_check(
        "AVD#1",
        ClientParams {
            proto: "tcp".to_string(),
            target: "10.0.2.2:{port}".to_string(),
            payload_size: 1024,
            expect_eof: false,
            timeout_ms: 5000,
        },
    )
    .await?;

    ctx.log_step("AVD#2", "AND", "Sends 1KB TCP to 10.0.2.2:{port}");
    ctx.run_client_check(
        "AVD#2",
        ClientParams {
            proto: "tcp".to_string(),
            target: "10.0.2.2:{port}".to_string(),
            payload_size: 1024,
            expect_eof: false,
            timeout_ms: 5000,
        },
    )
    .await?;

    ctx.log_step("Host", "THEN", "Receives 2KB TCP data total");
    Ok(())
}
