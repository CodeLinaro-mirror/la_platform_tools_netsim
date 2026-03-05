use anyhow::Result;

use super::{executor::ClientParams, TestContext};

pub fn run_suite(ctx: &mut TestContext) -> Result<()> {
    scenario_tcp_echo(ctx)?;
    scenario_udp_echo(ctx)?;
    scenario_gateway_performance(ctx)?;
    scenario_host_hangup(ctx)?;
    // scenario_performance(ctx)?;
    Ok(())
}

fn scenario_gateway_performance(ctx: &mut TestContext) -> Result<()> {
    println!("\nSCENARIO: Gateway Performance");
    println!("GIVEN the client is configured to reach gateway at {}", ctx.gateway_ip);

    // For Gateway Performance, we assume the host is running a server on the
    // gateway IP. Since we can't easily convince the host to bind ONLY to the
    // gateway IP without knowing it exactly (and 0.0.0.0 covers it), we can
    // reuse start_server(0) which binds 0.0.0.0. The client will then try to
    // connect to `ctx.gateway_ip:server_port`.

    println!("GIVEN a server running on the Host (0.0.0.0)");
    let port = ctx.start_server(0)?;
    println!("  -> Server assigned port {}", port);

    println!("WHEN the client connects to {}:{} via TCP", ctx.gateway_ip, port);
    // Note: We use ctx.gateway_ip here instead of ctx.target_ip
    let target = format!("{}:{}", ctx.gateway_ip, port);

    let result = ctx.run_client_check(ClientParams {
        proto: "tcp".to_string(),
        target,
        payload_size: 500 * 1024 * 1024, // 500MB
        expect_eof: false,
        timeout_ms: 60000,
    })?;

    println!("THEN the client should verify connectivity to the gateway");
    ctx.assert_success(result);
    Ok(())
}

pub fn list_scenarios() {
    println!("Available Scenarios:");
    println!("  - tcp_echo: Basic TCP Echo Test");
    println!("  - udp_echo: Basic UDP Echo Test");
    println!("  - host_hangup: Host Hangup Test (TCP)");
    println!("  - gateway_performance: Gateway Connectivity and Throughput Test");
}

fn scenario_tcp_echo(ctx: &mut TestContext) -> Result<()> {
    println!("\nSCENARIO: Basic TCP Echo");
    println!("GIVEN a server running on a random port");
    let port = ctx.start_server(0)?;
    println!("  -> Server assigned port {}", port);

    println!("WHEN the client connects to {}:{} via TCP", ctx.target_ip, port);
    let target = format!("{}:{}", ctx.target_ip, port);
    let result = ctx.run_client_check(ClientParams {
        proto: "tcp".to_string(),
        target,
        payload_size: 1024,
        expect_eof: false,
        timeout_ms: 1000,
    })?;

    println!("THEN the client should verify the echo");
    ctx.assert_success(result);
    Ok(())
}

fn scenario_udp_echo(ctx: &mut TestContext) -> Result<()> {
    println!("\nSCENARIO: Basic UDP Echo");
    println!("GIVEN a server running on a random port");
    let port = ctx.start_server(0)?;
    println!("  -> Server assigned port {}", port);

    println!("WHEN the client connects to {}:{} via UDP", ctx.target_ip, port);
    let target = format!("{}:{}", ctx.target_ip, port);
    let result = ctx.run_client_check(ClientParams {
        proto: "udp".to_string(),
        target,
        payload_size: 512,
        expect_eof: false,
        timeout_ms: 1000,
    })?;

    println!("THEN the client should verify the echo");
    ctx.assert_success(result);
    Ok(())
}

fn scenario_host_hangup(_ctx: &mut TestContext) -> Result<()> {
    println!("\nSCENARIO: Host Hangup (TCP)");
    println!("INFO: Skipping Host Hangup complex test for now (requires server control)");
    Ok(())
}

#[allow(dead_code)]
fn scenario_performance(ctx: &mut TestContext) -> Result<()> {
    println!("\nSCENARIO: Performance (TCP Large Payload)");
    println!("GIVEN a server running on a random port");
    let port = ctx.start_server(0)?;
    println!("  -> Server assigned port {}", port);

    println!("WHEN the client transfers 1MB to {}:{} via TCP", ctx.target_ip, port);
    let target = format!("{}:{}", ctx.target_ip, port);
    let result = ctx.run_client_check(ClientParams {
        proto: "tcp".to_string(),
        target,
        payload_size: 1024 * 1024, // 1MB
        expect_eof: false,
        timeout_ms: 5000,
    })?;

    println!("THEN the transfer should complete successfully");
    ctx.assert_success(result);
    Ok(())
}
