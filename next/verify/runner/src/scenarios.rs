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

/// Programmatic entry point for the entire integration test suite.
pub async fn run_suite(ctx: &mut TestContext) -> Result<()> {
    let mut features = features::Features::<TestContext>::new();
    crate::adb_steps::register_steps(&mut features);
    crate::android_steps::register_steps(&mut features);
    crate::host_steps::register_steps(&mut features);
    crate::netsim_steps::register_steps(&mut features);
    features.execute_from_memory(include_str!("../tests/features/echo.feature"), ctx).await;
    ctx.reset_actors().await?;

    features
        .execute_from_memory(include_str!("../tests/features/wifi_service_discovery.feature"), ctx)
        .await;
    ctx.reset_actors().await?;

    features
        .execute_from_memory(include_str!("../tests/features/gateway_performance.feature"), ctx)
        .await;
    ctx.reset_actors().await?;

    features
        .execute_from_memory(include_str!("../tests/features/multi_step_coordination.feature"), ctx)
        .await;
    ctx.reset_actors().await?;

    features
        .execute_from_memory(include_str!("../tests/features/multi_avd_echo.feature"), ctx)
        .await;
    ctx.reset_actors().await?;

    features.execute_from_memory(include_str!("../tests/features/nsd.feature"), ctx).await;
    ctx.reset_actors().await?;

    features.execute_from_memory(include_str!("../tests/features/p2p.feature"), ctx).await;

    Ok(())
}
