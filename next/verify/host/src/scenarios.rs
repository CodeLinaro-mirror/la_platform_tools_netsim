// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! # Netsim HDD Scenarios
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

use crate::orchestrator::TestContext;

async fn run_feature(
    features: &mut features::Features<TestContext>,
    ctx: &mut TestContext,
    name: &str,
    content: &str,
) -> Result<(), String> {
    if let Some(f) = &ctx.filter {
        if !name.contains(f) {
            return Ok(());
        }
    }
    if ctx.is_dry_run {
        features.dry_run_list(content);
        return Ok(());
    }

    for is_retry in [false, true] {
        if is_retry {
            ctx.reset_actors(true).await?;
            println!("INFO: Retrying feature in isolation...");
        }

        match features.execute_from_memory(content, ctx).await {
            Ok(_) => {
                if is_retry {
                    println!("INFO: Feature succeeded on retry after hard reset.");
                }
                ctx.reset_actors(false).await?;
                break;
            }
            Err(e) => {
                if !is_retry {
                    println!("WARN: Feature failed: {}. Attempting isolation recovery...", e);
                } else {
                    println!("ERROR: Feature failed again on retry: {}", e);
                    // Reset actors again (hard) to be safe for next scenarios if keep_going is true
                    ctx.reset_actors(true).await?;
                    return Err(e);
                }
            }
        }
    }
    Ok(())
}

pub async fn run_suite(
    ctx: &mut TestContext,
    mut features: features::Features<TestContext>,
    spec_dir: Option<String>,
) -> Result<(), String> {
    let dir_path = match spec_dir {
        Some(path) => path,
        None => return Err("No spec directory provided. Use --spec-dir.".to_string()),
    };
    let dir = std::path::Path::new(&dir_path);
    if !dir.is_dir() {
        return Err(format!("Features directory not found: {}", dir_path));
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "feature") {
            entries.push(path);
        }
    }
    // Sort for deterministic order
    entries.sort();

    let mut error_messages = Vec::new();

    for path in entries {
        let filename = path.file_name().unwrap().to_string_lossy().into_owned();
        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        match run_feature(&mut features, ctx, &filename, &content).await {
            Ok(_) => {}
            Err(e) => {
                error_messages.push(format!("{}: {:?}", filename, e));
                if !ctx.keep_going {
                    return Err(e);
                }
            }
        }
    }

    if !error_messages.is_empty() {
        println!("\n--- Test Failures Summary ---");
        for msg in &error_messages {
            println!("{}", msg);
        }
        return Err("Some specifications failed".to_string());
    }

    Ok(())
}
