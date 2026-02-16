mod executor;
mod scenarios;

use anyhow::Result;
use executor::{AndroidExecutor, Executor, LocalExecutor};

pub async fn run_local(
    wifi_tap: Option<String>,
    wifi_cvd_tap: bool,
    gateway_ip_arg: Option<String>,
) -> Result<()> {
    // Determine Gateway IP
    // Priority: Explicit arg > CVD TAP flag > Default Slirp
    let gateway_ip = if let Some(ip) = gateway_ip_arg {
        ip
    } else if wifi_cvd_tap {
        "192.168.96.1".to_string()
    } else if let Some(tap) = &wifi_tap {
        if tap.contains("cvd-etap") {
            "192.168.96.1".to_string()
        } else {
            // Unknown TAP configuration, fallback to Slirp or likely fail.
            // User should provide --gateway-ip if using custom TAP.
            println!("[ntest] WARN: Using custom TAP without explicit --gateway-ip. Defaulting to 10.0.2.2 (Slirp) which might be wrong.");
            "10.0.2.2".to_string()
        }
    } else {
        "10.0.2.2".to_string()
    };

    let mut ctx = TestContext {
        executor: Box::new(LocalExecutor),
        server_process: None,
        target_ip: "127.0.0.1".to_string(),
        gateway_ip: gateway_ip.clone(),
    };
    scenarios::run_suite(&mut ctx)?;
    Ok(())
}

pub async fn run_android(
    android_bin: Option<String>,
    netsim_bin: Option<String>,
    netsim_args: Option<String>,
) -> Result<()> {
    // Construct netsim_args to pass to daemon if not already provided
    // For now, we just use what was provided.

    let gateway_ip = "10.0.2.2".to_string(); // In Android emulator, Host is 10.0.2.2 usually.

    let mut executor = AndroidExecutor::new(None, android_bin, netsim_bin, netsim_args)
        .ok()
        .ok_or(anyhow::anyhow!("Failed to create AndroidExecutor"))?;

    // Always manage lifecycle in Android mode
    executor.launch_netsim()?;
    executor.launch_emulator()?;
    executor.push_binary()?;
    executor.wait_for_network()?;

    let mut ctx = TestContext {
        executor: Box::new(executor),
        server_process: None,
        target_ip: "10.0.2.2".to_string(), // Guest connects to Host Alias
        gateway_ip,
    };
    scenarios::run_suite(&mut ctx)?;
    Ok(())
}

pub struct TestContext {
    executor: Box<dyn Executor>,
    server_process: Option<std::process::Child>,
    pub target_ip: String,
    pub gateway_ip: String,
}

impl TestContext {
    pub fn start_server(&mut self, port: u16) -> Result<u16> {
        if self.server_process.is_some() {
            self.stop_server()?;
        }
        let (child, assigned_port) = self.executor.spawn_server(port)?;
        self.server_process = Some(child);
        // Give server a moment to start (already waited for port in spawn_server, but
        // extra safety?)
        // std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(assigned_port)
    }

    pub fn stop_server(&mut self) -> Result<()> {
        if let Some(mut child) = self.server_process.take() {
            child.kill()?;
            child.wait()?;
        }
        Ok(())
    }

    pub fn run_client_check(&self, params: executor::ClientParams) -> Result<std::process::Output> {
        self.executor.run_client(params)
    }

    pub fn assert_success(&self, output: std::process::Output) {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stdout.is_empty() {
            println!("{}", stdout);
        } else {
            println!("[ntest runner] STDOUT is empty. Len: {}", output.stdout.len());
        }
        if !stderr.is_empty() {
            eprintln!("{}", stderr);
        } else {
            println!("[ntest runner] STDERR is empty. Len: {}", output.stderr.len());
        }

        if !output.status.success() {
            panic!("Client check failed:\nSTDOUT: {}\nSTDERR: {}", stdout, stderr);
        }
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        let _ = self.stop_server();
    }
}

pub fn list_scenarios() {
    scenarios::list_scenarios();
}
