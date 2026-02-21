//! # Netsim Test Orchestrator
//!
//! This module implements the central Control Plane for `ntest`. It manages the
//! lifecycle of test actors (Guest Emulators and the Host), handles ADB-based
//! device discovery, and coordinates the execution of BDD-style scenarios.
//!
//! ## Execution Model
//! 1. **Discovery**: Scans for attached devices, installs the agent, and
//!    establishes a feedback tunnel.
//! 2. **Context**: Maintains the `TestContext`, which tracks the "Definition of
//!    Reality" (ports, IPs, targets).
//! 3. **Orchestration**: Iterates through scenarios, resolving placeholders and
//!    routing commands to actors.

mod executor;
mod scenarios;

use anyhow::{Context, Result};
#[allow(unused_imports)]
pub use executor::LABEL_WIDTH;
use executor::{AndroidExecutor, Executor, Throughput};
use tokio_util::sync::CancellationToken;

/// Orchestrates Android integration tests by discovering devices and running
/// the suite.
pub async fn run_android(
    android_home: Option<String>,
    netsim_path: Option<String>,
    netsim_args: Option<String>,
    apk_path: Option<String>,
    gateway_ip: Option<String>,
    dry_run: bool,
) -> Result<()> {
    let mut ctx = TestContext {
        executors: vec![],
        server_token: None,
        target_ip: "10.0.2.2".to_string(),
        gateway_ip: gateway_ip.unwrap_or_else(|| "10.0.2.2".to_string()),
        is_dry_run: dry_run,
        last_port: None,
        android_home,
        apk_path,
        netsim_path,
        netsim_args,
    };
    scenarios::run_suite(&mut ctx).await?;
    Ok(())
}

/// Global context for the test suite, holding executors and shared
/// configuration.
pub struct TestContext {
    executors: Vec<Box<dyn Executor>>,
    server_token: Option<CancellationToken>,
    pub target_ip: String,
    pub gateway_ip: String,
    pub is_dry_run: bool,
    pub last_port: Option<u16>,

    // Path configuration for discovery
    pub android_home: Option<String>,
    pub apk_path: Option<String>,
    pub netsim_path: Option<String>,
    pub netsim_args: Option<String>,
}

impl TestContext {
    /// Start the host-side echo server.
    pub async fn start_server(&mut self, port: u16) -> Result<u16> {
        let assigned_port = if self.is_dry_run {
            if port == 0 {
                12345
            } else {
                port
            }
        } else {
            self.stop_server();
            let token = CancellationToken::new();
            let p = crate::server::run_server(port, token.clone()).await?;
            self.server_token = Some(token);
            p
        };

        self.last_port = Some(assigned_port);
        Ok(assigned_port)
    }

    pub fn stop_server(&mut self) {
        if let Some(token) = self.server_token.take() {
            token.cancel();
        }
    }

    /// Execute a network client task on a specific actor.
    pub async fn run_client_check(
        &self,
        actor: &str,
        mut params: executor::ClientParams,
    ) -> Result<Option<Throughput>> {
        if self.is_dry_run {
            return Ok(None);
        }
        params.target = self.resolve_placeholders(&params.target);
        self.executors[self.resolve_actor_index(actor)?].run_client(params).await
    }

    /// Execute an arbitrary BDD step on a specific actor.
    pub async fn execute_step(&self, actor: &str, step: &str) -> Result<()> {
        if self.is_dry_run {
            return Ok(());
        }
        let resolved = self.resolve_placeholders(step);
        self.executors[self.resolve_actor_index(actor)?].execute_step(&resolved).await
    }

    /// Run a benchmark with multiple samples and log iperf3-style output.
    pub async fn run_benchmark(
        &self,
        actor: &str,
        mut params: executor::ClientParams,
        samples: usize,
    ) -> Result<()> {
        let idx = self.resolve_actor_index(actor)?;
        params.target = self.resolve_placeholders(&params.target);
        if self.is_dry_run {
            return Ok(());
        }

        self.log_info(actor, "[ ID] Interval           Transfer     Bitrate");
        let mut total_bytes = 0;
        let mut total_duration = std::time::Duration::from_secs(0);

        self.executors[idx].set_silent(true);
        for i in 0..samples {
            let start_time = total_duration.as_secs_f64();
            let params_clone = executor::ClientParams {
                payload_size: params.payload_size,
                proto: params.proto.clone(),
                target: params.target.clone(),
                expect_eof: params.expect_eof,
                timeout_ms: params.timeout_ms,
            };
            if let Some(tp) = self.run_client_check(actor, params_clone).await? {
                total_bytes += tp.bytes;
                total_duration += tp.duration;
                self.log_iperf_line(actor, tp, start_time);
            }
            if i < samples - 1 {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
        self.executors[idx].set_silent(false);

        // Summary Line
        let tag = self.actor_tag(actor);
        println!("{:<6} {} - - - - - - - - - - - - - - - - - - - - - - - - -", "INFO", tag);
        self.log_iperf_line(
            actor,
            Throughput { bytes: total_bytes, duration: total_duration },
            0.0,
        );
        Ok(())
    }

    pub async fn reset_agent(&mut self) -> Result<()> {
        if self.is_dry_run {
            return Ok(());
        }
        for exec in self.executors.iter_mut() {
            exec.reset_agent().await?;
        }
        Ok(())
    }

    /// Centralized logging for BDD steps and telemetry.
    pub fn log_step(&self, actor: &str, verb: &str, msg: &str) {
        let tag = self.actor_tag(actor);
        println!("{:<6} {} {}", verb, tag, self.resolve_placeholders(msg));
    }

    /// Logs observational runtime information (telemetry, progress).
    pub fn log_info(&self, actor: &str, msg: &str) {
        self.log_step(actor, "INFO", msg);
    }

    /// Resolves the human-readable tag for an actor (e.g., "@emulator-5554").
    fn actor_tag(&self, actor: &str) -> String {
        let label = if self.is_dry_run || actor == "Host" || actor == "adb" {
            actor.to_string()
        } else {
            self.resolve_actor_label(actor)
        };
        format!("@{:width$}", label, width = executor::LABEL_WIDTH)
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
            "{:<6} {} [  5] {:5.2}-{:5.2} sec  {:6.2} {:6}  {:6.2} {}",
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

    fn resolve_actor_index(&self, actor: &str) -> Result<usize> {
        if actor == "AVD" || actor == "AVD#1" {
            if self.executors.is_empty() {
                if self.is_dry_run {
                    return Ok(0);
                }
                anyhow::bail!("No devices found");
            }
            Ok(0)
        } else if let Some(idx_str) = actor.strip_prefix("AVD#") {
            let idx = idx_str.parse::<usize>()? - 1;
            if self.is_dry_run {
                return Ok(idx);
            }
            if idx >= self.executors.len() {
                anyhow::bail!(
                    "Requested {}, but only {} devices found",
                    actor,
                    self.executors.len()
                );
            }
            Ok(idx)
        } else {
            self.executors
                .iter()
                .position(|e| e.get_label() == actor)
                .or_else(|| if self.is_dry_run { Some(0) } else { None })
                .ok_or_else(|| anyhow::anyhow!("Unknown actor: {}", actor))
        }
    }

    fn resolve_actor_label(&self, actor: &str) -> String {
        match self.resolve_actor_index(actor) {
            Ok(i) if i < self.executors.len() => self.executors[i].get_label(),
            _ => actor.to_string(), // Keep placeholder if dry-run or not found
        }
    }

    /// Resolves BDD placeholders (e.g., {port}, {target}) to runtime values.
    fn resolve_placeholders(&self, s: &str) -> String {
        // In dry-run mode, we preserve placeholders for architectural documentation.
        if self.is_dry_run {
            return s.to_string();
        }

        let port_str = self.last_port.map(|p| p.to_string()).unwrap_or_else(|| "0".to_string());
        let target = format!("{}:{}", self.target_ip, port_str);
        let gateway = format!("{}:{}", self.gateway_ip, port_str);

        let mut res = s
            .to_string()
            .replace("{port}", &port_str)
            .replace("{target}", &target)
            .replace("{gateway_target}", &gateway)
            .replace("{gateway_ip}", &self.gateway_ip);

        // Resolve @AVD#N placeholders to actual runtime labels.
        for i in (1..=std::cmp::max(2, self.executors.len())).rev() {
            let tag = format!("@AVD#{}", i);
            let label =
                self.executors.get(i - 1).map(|e| e.get_label()).unwrap_or_else(|| tag.clone());
            res = res.replace(&tag, &format!("@{}", label));
        }

        // Fallback for single-device scenarios using @AVD.
        let default_label =
            self.executors.get(0).map(|e| e.get_label()).unwrap_or_else(|| "AVD".to_string());
        res.replace("@AVD", &format!("@{}", default_label))
    }

    /// Discovers attached Android devices and prepares them for testing.
    pub async fn discover_devices(&mut self, expected: usize) -> Result<()> {
        self.log_step("adb", "GIVEN", &format!("Has {} or more attached devices", expected));
        if self.is_dry_run || self.executors.len() >= expected {
            return Ok(());
        }

        let adb = AndroidExecutor::find_adb(self.android_home.as_deref());
        let out = std::process::Command::new(&adb)
            .arg("devices")
            .output()
            .context("Failed to execute adb devices")?;

        for line in String::from_utf8_lossy(&out.stdout).lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 || parts[1] != "device" {
                continue;
            }

            let serial = parts[0].to_string();
            if self.executors.iter().any(|e| e.get_serial().as_ref() == Some(&serial)) {
                continue;
            }

            let mut exec = AndroidExecutor::new(Some(serial), adb.clone(), self.apk_path.clone())?;
            self.identify_avd(&adb, &mut exec);

            exec.launch_netsim(&self.netsim_path, &self.netsim_args)?;

            self.log_info(&exec.get_label(), "Installing ntest-agent APK...");
            exec.install_apk()?;
            exec.setup_feedback()?;

            // Stabilization delay for ADB reverse tunnels.
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            self.log_info(&exec.get_label(), "Launching NTest instrumentation agent...");
            exec.launch_agent()?;
            exec.wait_for_feedback()?;

            self.executors.push(Box::new(exec));
            if self.executors.len() >= expected {
                break;
            }
        }

        if self.executors.len() < expected {
            anyhow::bail!(
                "Discovery failed: Required {} devices, but only found {}",
                expected,
                self.executors.len()
            );
        }
        Ok(())
    }

    /// Attempts to identify the AVD name for a given device serial.
    fn identify_avd(&self, adb: &str, exec: &mut AndroidExecutor) {
        let serial = exec.serial.as_ref().unwrap();
        if let Ok(out) = std::process::Command::new(adb)
            .arg("-s")
            .arg(serial)
            .arg("emu")
            .arg("avd")
            .arg("name")
            .stderr(std::process::Stdio::null())
            .output()
        {
            let name = String::from_utf8_lossy(&out.stdout)
                .lines()
                .last()
                .unwrap_or("")
                .trim()
                .to_string();
            if !name.is_empty() && name != "OK" {
                exec.avd_name = Some(name);
            }
        }
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        self.stop_server();
    }
}

pub async fn list_scenarios() {
    let mut ctx = TestContext {
        executors: vec![],
        server_token: None,
        target_ip: "10.0.2.2".to_string(),
        gateway_ip: "10.0.2.2".to_string(),
        is_dry_run: true,
        last_port: None,
        android_home: None,
        apk_path: None,
        netsim_path: None,
        netsim_args: None,
    };
    scenarios::run_suite(&mut ctx).await.unwrap();
}
