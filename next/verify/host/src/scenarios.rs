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

pub async fn run_suite(
    ctx: &mut TestContext,
    mut features: features::Features<TestContext>,
    spec_dir: Option<String>,
) -> Result<()> {
    if let Some(dir) = spec_dir {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "feature") {
                entries.push(path);
            }
        }
        // Sort for deterministic order
        entries.sort();

        for path in entries {
            let filename = path.file_name().unwrap().to_string_lossy().into_owned();
            let content = std::fs::read_to_string(&path)?;
            run_feature(&mut features, ctx, &filename, &content).await?;
        }
    } else {
        anyhow::bail!("No spec directory provided. Use --spec-dir.");
    }

    Ok(())
}
