//! # Netsim Test Orchestrator
//!
//! This module implements the central Control Plane for `ntest`. It manages the
//! lifecycle of test actors (Guest Emulators and the Host), handles ADB-based
//! device discovery, and coordinates the execution of BDD-style scenarios.

// Modules are now declared in main.rs (siblings)

#[allow(unused_imports)]
pub use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use anyhow::Result;

use crate::{
    adb_steps::AdbWorld,
    android_steps::{AndroidDevice, AndroidWorld},
    host_steps::HostWorld,
    netsim_steps::NetsimWorld,
    scenarios,
    types::{ClientParams, Throughput, LABEL_WIDTH},
};

// ...

/// Orchestrates Android integration tests by discovering devices and running
/// the suite.
pub async fn run_android(
    android_home: Option<String>,
    netsim_path: Option<String>,
    netsim_args: Option<String>,
    apk_path: Option<String>,
    gateway_ip: Option<String>,
    filter: Option<String>,
    dry_run: bool,
) -> Result<()> {
    let host = HostWorld::new(dry_run);
    let adb = AdbWorld::new(android_home, apk_path, netsim_path, netsim_args);
    let netsim = NetsimWorld {};

    let mut ctx = TestContext {
        android: AndroidWorld::new(),
        host,
        adb,
        netsim,
        target_ip: "10.0.2.2".to_string(),
        gateway_ip: gateway_ip.unwrap_or_else(|| "10.0.2.2".to_string()),
        filter,
        is_dry_run: dry_run,
        variables: HashMap::new(),
    };
    scenarios::run_suite(&mut ctx).await?;
    Ok(())
}

pub async fn list_scenarios() {
    let mut ctx = TestContext {
        android: AndroidWorld::new(),
        host: HostWorld::new(true),
        adb: AdbWorld::new(None, None, None, None),
        netsim: NetsimWorld {},
        target_ip: "10.0.2.2".to_string(),
        gateway_ip: "10.0.2.2".to_string(),
        filter: None,
        is_dry_run: true,
        variables: HashMap::new(),
    };
    scenarios::run_suite(&mut ctx).await.unwrap();
}

/// Global context for the test suite, holding actors and shared
/// configuration.
pub struct TestContext {
    pub android: AndroidWorld,
    pub host: HostWorld,
    pub adb: AdbWorld,
    pub netsim: NetsimWorld,

    pub target_ip: String,
    pub gateway_ip: String,
    pub filter: Option<String>,
    pub is_dry_run: bool,
    pub variables: HashMap<String, String>,
}

impl features::World for TestContext {
    fn reset(&mut self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            let keys: Vec<String> = self.android.devices.keys().cloned().collect();
            for key in keys {
                if let Some(agent) = self.android.devices.get_mut(&key) {
                    // "resets world" is defined in LifecycleSteps.kt
                    if let Err(e) = agent.execute_step("resets world").await {
                        println!("WARN: Failed to reset world on {}: {}", key, e);
                    }
                }
            }
        })
    }
}

impl TestContext {
    // Helper to resolve generic actor lookups for the engine
    // Since we removed TestActor enum, we need to dispatch manually if needed
    // But for now, most steps are specific to an actor type.

    // Legacy support for dry-run resolution (mostly used for label resolving)

    pub fn get_android_actor(&self, key: &str) -> Option<&AndroidDevice> {
        self.android.get(key)
    }

    pub fn get_android_actor_mut(&mut self, key: &str) -> Option<&mut AndroidDevice> {
        self.android.get_mut(key)
    }

    pub fn get_all_serials(&self) -> HashSet<String> {
        self.android.get_all_serials()
    }

    pub fn register_android_actor(&mut self, agent: AndroidDevice) {
        self.android.register(agent);
    }

    pub fn set_variable(&mut self, key: &str, value: String) {
        println!("    <- SET {}={}", key, value);
        self.variables.insert(key.to_string(), value);
    }

    /// Execute a network client task on a specific actor.
    pub async fn run_client_check(
        &mut self,
        actor: &str,
        mut params: ClientParams,
    ) -> Result<Option<Throughput>> {
        if self.is_dry_run {
            return Ok(None);
        }
        params.target = self.resolve_placeholders(&params.target);

        if actor == "@host" {
            return self.host.run_client(params).await;
        } else if actor == "@adb" {
            return self.adb.run_client(params).await;
        } else if actor == "@netsim" {
            return self.netsim.run_client(params).await;
        }

        if let Some(agent) = self.get_android_actor_mut(actor) {
            agent.run_client(params).await
        } else {
            anyhow::bail!("Unknown actor: {}", actor)
        }
    }

    // Removed execute_step helper

    /// Run a benchmark with multiple samples and log iperf3-style output.
    pub async fn run_benchmark(
        &mut self,
        actor: &str,
        mut params: ClientParams,
        samples: usize,
    ) -> Result<()> {
        // Resolve target before cloning
        params.target = self.resolve_placeholders(&params.target);
        if self.is_dry_run {
            return Ok(());
        }

        self.log_info(actor, "[ ID] Interval           Transfer     Bitrate");
        let mut total_bytes = 0;
        let mut total_duration = std::time::Duration::from_secs(0);

        // Scope for mutable borrow of actor
        {
            {
                if actor == "@host" {
                    self.host.set_silent(true);
                } else if actor == "@adb" {
                    self.adb.set_silent(true);
                } else if actor == "@netsim" {
                    self.netsim.set_silent(true);
                } else {
                    let agent = self
                        .get_android_actor_mut(actor)
                        .ok_or_else(|| anyhow::anyhow!("Unknown actor: {}", actor))?;
                    agent.set_silent(true);
                }
            }
        }

        for i in 0..samples {
            let start_time = total_duration.as_secs_f64();
            let params_clone = ClientParams {
                payload_size: params.payload_size,
                proto: params.proto.clone(),
                target: params.target.clone(),
                expect_eof: params.expect_eof,
                timeout_ms: params.timeout_ms,
            };

            // run_client_check handles borrow internally
            if let Some(tp) = self.run_client_check(actor, params_clone).await? {
                total_bytes += tp.bytes;
                total_duration += tp.duration;
                self.log_iperf_line(actor, tp, start_time);
            }
            if i < samples - 1 {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        {
            {
                if actor == "@host" {
                    self.host.set_silent(false);
                } else if actor == "@adb" {
                    self.adb.set_silent(false);
                } else if actor == "@netsim" {
                    self.netsim.set_silent(false);
                } else {
                    let agent = self
                        .get_android_actor_mut(actor)
                        .ok_or_else(|| anyhow::anyhow!("Unknown actor: {}", actor))?;
                    agent.set_silent(false);
                }
            }
        }

        // Summary Line
        let tag = self.actor_tag(actor);
        println!("    {:<6} {} - - - - - - - - - - - - - - - - - - - - - - - - -", "INFO", tag);
        self.log_iperf_line(
            actor,
            Throughput { bytes: total_bytes, duration: total_duration },
            0.0,
        );
        Ok(())
    }

    pub async fn reset_actors(&mut self) -> Result<()> {
        if self.is_dry_run {
            return Ok(());
        }
        self.host.reset_actor().await?;
        self.adb.reset_actor().await?;
        self.netsim.reset_actor().await?;
        for agent in self.android.devices.values_mut() {
            agent.reset_actor().await?;
        }
        Ok(())
    }

    /// Centralized logging for BDD steps and telemetry.
    pub fn log_step(&self, actor: &str, verb: &str, msg: &str) {
        let tag = self.actor_tag(actor);
        let prefix = if verb == "INFO" { "INFO" } else { "->" };
        let resolved = self.resolve_placeholders(msg);
        let capitalized = if let Some(first) = resolved.chars().next() {
            format!("{}{}", first.to_uppercase(), &resolved[first.len_utf8()..])
        } else {
            resolved
        };
        println!("    {:<6} {} {}", prefix, tag, capitalized);
    }

    /// Logs observational runtime information (telemetry, progress).
    pub fn log_info(&self, actor: &str, msg: &str) {
        self.log_step(actor, "INFO", msg);
    }

    /// Resolves the human-readable tag for an actor (e.g., "@emulator-5554").
    fn actor_tag(&self, actor: &str) -> String {
        let mut label = if actor == "@host" {
            self.host.get_label()
        } else if actor == "@adb" {
            self.adb.get_label()
        } else if actor == "@netsim" {
            self.netsim.get_label()
        } else if let Some(agent) = self.get_android_actor(actor) {
            agent.get_label()
        } else {
            // In dry run, actor might be "@avd:1"
            // If we mocked devices, get_android_actor would return Some.
            // If we didn't, we fall back here.
            actor.to_string()
        };
        if !label.starts_with('@') {
            label = format!("@{}", label);
        }
        format!("{:width$}", label, width = LABEL_WIDTH)
    }

    fn log_iperf_line(&self, actor: &str, tp: Throughput, start: f64) {
        let end = start + tp.duration.as_secs_f64();
        let bits_sec = (tp.bytes as f64 * 8.0) / tp.duration.as_secs_f64();

        let (trans, t_unit) = if tp.bytes >= 1024 * 1024 {
            (tp.bytes as f64 / 1048576.0, "MBytes")
        } else if tp.bytes >= 1024 {
            (tp.bytes as f64 / 1024.0, "KBytes")
        } else {
            (tp.bytes as f64, "Bytes")
        };

        let (rate, r_unit) = if bits_sec >= 1_000_000.0 {
            (bits_sec / 1_000_000.0, "Mbits/sec")
        } else if bits_sec >= 1_000.0 {
            (bits_sec / 1_000.0, "Kbits/sec")
        } else {
            (bits_sec, "bits/sec")
        };

        println!(
            "    {:<6} {} [  5] {:5.2}-{:5.2} sec  {:6.2} {:6}  {:6.2} {}",
            "INFO",
            self.actor_tag(actor),
            start,
            end,
            trans,
            t_unit,
            rate,
            r_unit
        );
    }

    /// Resolves BDD placeholders (e.g., {port}, {target}) to runtime values.
    pub fn resolve_placeholders(&self, s: &str) -> String {
        let mut res = s.to_string();

        // Resolve variables from HashMap
        for (key, val) in &self.variables {
            res = res.replace(&format!("{{{}}}", key), val);
        }

        // Helpers
        if let Some(port) = self.variables.get("port") {
            let target = if port.contains(':') {
                port.clone()
            } else {
                format!("{}:{}", self.target_ip, port)
            };
            res = res.replace("{target}", &target);
        }

        let target = format!("{}:0", self.target_ip); // Default target if port unknown
        res = res.replace("{target}", &target);

        // Resolve @avd:N placeholders
        for (i, key) in self.android.avd_keys.iter().enumerate() {
            let n = i + 1;
            let tag = format!("@avd:{}", n);
            let label = self.android.devices.get(key).map(|e| e.get_label()).unwrap_or(key.clone());
            res = res.replace(&tag, &format!("@{}", label));
        }

        // Fallback for @avd
        if !self.android.avd_keys.is_empty() {
            let label = self
                .android
                .devices
                .get(&self.android.avd_keys[0])
                .map(|e| e.get_label())
                .unwrap_or("@avd:1".to_string());
            res = res.replace("@avd", &format!("@{}", label));
        }

        res
    }
}
