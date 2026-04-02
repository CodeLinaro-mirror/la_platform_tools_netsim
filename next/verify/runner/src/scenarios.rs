// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

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

use crate::orchestrator::TestContext;

async fn run_feature(
    features: &mut features::Features<TestContext>,
    ctx: &mut TestContext,
    name: &str,
    content: &str,
) -> Result<()> {
    if let Some(f) = &ctx.filter {
        if !name.contains(f) {
            return Ok(());
        }
    }
    if ctx.is_dry_run {
        features.dry_run_list(content);
        return Ok(());
    }

    features.execute_from_memory(content, ctx).await;
    ctx.reset_actors().await?;
    Ok(())
}

/// Programmatic entry point for the entire integration test suite.
pub async fn run_suite(ctx: &mut TestContext) -> Result<()> {
    let mut features = features::Features::<TestContext>::new();
    crate::adb_steps::register_steps(&mut features);
    crate::android_steps::register_steps(&mut features);
    crate::host_steps::register_steps(&mut features);
    crate::netsim_steps::register_steps(&mut features);

    run_feature(&mut features, ctx, "echo.feature", include_str!("../tests/features/echo.feature"))
        .await?;

    run_feature(
        &mut features,
        ctx,
        "wifi_service_discovery.feature",
        include_str!("../tests/features/wifi_service_discovery.feature"),
    )
    .await?;

    run_feature(
        &mut features,
        ctx,
        "gateway_performance.feature",
        include_str!("../tests/features/gateway_performance.feature"),
    )
    .await?;

    run_feature(
        &mut features,
        ctx,
        "multi_step_coordination.feature",
        include_str!("../tests/features/multi_step_coordination.feature"),
    )
    .await?;

    run_feature(
        &mut features,
        ctx,
        "multi_avd_echo.feature",
        include_str!("../tests/features/multi_avd_echo.feature"),
    )
    .await?;

    run_feature(&mut features, ctx, "nsd.feature", include_str!("../tests/features/nsd.feature"))
        .await?;

    run_feature(&mut features, ctx, "p2p.feature", include_str!("../tests/features/p2p.feature"))
        .await?;

    run_feature(
        &mut features,
        ctx,
        "bluetooth_advertisements.feature",
        include_str!("../tests/features/bluetooth_advertisements.feature"),
    )
    .await?;

    run_feature(
        &mut features,
        ctx,
        "uwb_ranging.feature",
        include_str!("../tests/features/uwb_ranging.feature"),
    )
    .await?;

    run_feature(
        &mut features,
        ctx,
        "wifi_auth.feature",
        include_str!("../tests/features/wifi_auth.feature"),
    )
    .await?;

    Ok(())
}
