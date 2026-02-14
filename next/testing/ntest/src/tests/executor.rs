use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
};

pub trait Executor {
    fn spawn_server(&self, port: u16) -> anyhow::Result<(Child, u16)>;
    fn run_client(&self, params: ClientParams) -> anyhow::Result<std::process::Output>;
}

fn spawn_server_common(port: u16) -> anyhow::Result<(Child, u16)> {
    let exe = std::env::current_exe()?;
    let mut child = Command::new(exe)
        .arg("server")
        .arg("--port") // This port might be 0
        .arg(port.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("No stdout"))?;
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let mut assigned_port = 0;

    // Read until we find SERVER_PORT
    while reader.read_line(&mut line)? > 0 {
        if line.starts_with("SERVER_PORT=") {
            let port_str = line.trim().trim_start_matches("SERVER_PORT=");
            assigned_port = port_str.parse().unwrap_or(0);
            println!("[server] {}", line.trim());
            line.clear();
            break;
        }
        print!("[server] {}", line);
        line.clear();
    }

    if assigned_port == 0 {
        anyhow::bail!("Server failed to start or didn't print port");
    }

    // Spawn thread to forward rest
    std::thread::spawn(move || {
        while reader.read_line(&mut line).unwrap_or(0) > 0 {
            print!("[server] {}", line);
            line.clear();
        }
    });

    Ok((child, assigned_port))
}

#[derive(Default, Clone)]
pub struct ClientParams {
    pub proto: String,
    pub target: String,
    pub payload_size: usize,
    pub expect_eof: bool,
    pub timeout_ms: u64,
}

impl ClientParams {
    pub fn to_flags(&self) -> Vec<String> {
        let mut args = vec![
            "--proto".to_string(),
            self.proto.clone(),
            "--target".to_string(),
            self.target.clone(),
            "--payload-size".to_string(),
            self.payload_size.to_string(),
            "--timeout".to_string(),
            self.timeout_ms.to_string(),
        ];
        if self.expect_eof {
            args.push("--expect-eof".to_string());
        }
        args
    }
}

pub struct LocalExecutor;

impl Executor for LocalExecutor {
    fn spawn_server(&self, port: u16) -> anyhow::Result<(Child, u16)> {
        spawn_server_common(port)
    }

    fn run_client(&self, params: ClientParams) -> anyhow::Result<std::process::Output> {
        let exe = std::env::current_exe()?;
        Ok(Command::new(exe).arg("client").args(params.to_flags()).output()?)
    }
}

pub struct AndroidExecutor {
    pub adb_path: String,
    pub android_bin_path: Option<String>,
    pub serial: Option<String>,
    pub emulator_process: Option<Child>,
    pub netsim_bin: Option<String>,
    pub netsim_args: Option<String>,
    pub netsim_process: Option<Child>,
}

impl AndroidExecutor {
    pub fn new(
        serial: Option<String>,
        android_bin_path: Option<String>,
        netsim_bin: Option<String>,
        netsim_args: Option<String>,
    ) -> anyhow::Result<Self> {
        // Try to find adb in PATH or ANDROID_HOME
        let adb_path = if let Ok(path) = which::which("adb") {
            path.to_string_lossy().to_string()
        } else if let Ok(home) = std::env::var("ANDROID_HOME") {
            let path = std::path::Path::new(&home).join("platform-tools").join("adb");
            if path.exists() {
                path.to_string_lossy().to_string()
            } else {
                "adb".to_string()
            }
        } else if let Ok(home) = std::env::var("ANDROID_SDK_HOME") {
            let path = std::path::Path::new(&home).join("platform-tools").join("adb");
            if path.exists() {
                path.to_string_lossy().to_string()
            } else {
                "adb".to_string()
            }
        } else {
            "adb".to_string()
        };

        Ok(Self {
            adb_path,
            android_bin_path,
            serial,
            emulator_process: None,
            netsim_bin,
            netsim_args,
            netsim_process: None,
        })
    }

    fn adb_command(&self) -> Command {
        let mut cmd = Command::new(&self.adb_path);
        if let Some(s) = &self.serial {
            cmd.arg("-s").arg(s);
        }
        cmd
    }

    pub fn launch_emulator(&mut self) -> anyhow::Result<()> {
        // Check if a device is already connected
        let output = Command::new(&self.adb_path).arg("devices").output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("[ntest] 'adb devices' output:\n{}", stdout);

        // Simple parse: look for any line ending in "device"
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == "device" {
                let serial = parts[0];
                println!("[ntest] Found running emulator/device: {}. Skipping launch.", serial);
                self.serial = Some(serial.to_string());
                return Ok(());
            }
        }

        let emu_path = if let Ok(home) = std::env::var("ANDROID_SDK_HOME") {
            let path = std::path::Path::new(&home).join("emulator").join("emulator");
            if path.exists() {
                path.to_string_lossy().to_string()
            } else {
                "emulator".to_string()
            }
        } else if let Ok(home) = std::env::var("ANDROID_HOME") {
            let path = std::path::Path::new(&home).join("emulator").join("emulator");
            if path.exists() {
                path.to_string_lossy().to_string()
            } else {
                "emulator".to_string()
            }
        } else {
            "emulator".to_string()
        };

        // Detect AVD
        let output = Command::new(&emu_path).arg("-list-avds").output()?;
        let avds = String::from_utf8_lossy(&output.stdout);
        let first_avd = avds.lines().next().ok_or_else(|| anyhow::anyhow!("No AVDs found"))?;
        let avd_name = format!("@{}", first_avd);

        println!("[ntest] Launching emulator: {} with AVD: {}", emu_path, avd_name);
        let mut cmd = Command::new(emu_path);
        cmd.arg(&avd_name).arg("-no-window").stdout(Stdio::inherit()).stderr(Stdio::inherit());

        if let Some(args) = &self.netsim_args {
            cmd.arg("-netsim-args").arg(args);
        }

        let child = cmd.spawn()?;

        self.emulator_process = Some(child);
        self.wait_for_device_online()?;
        Ok(())
    }

    fn wait_for_device_online(&self) -> anyhow::Result<()> {
        println!("[ntest] Waiting for device to be online (adb wait-for-device)...");
        let status = self.adb_command().arg("wait-for-device").output()?;
        if !status.status.success() {
            anyhow::bail!("Failed to wait for device");
        }

        println!("[ntest] Device found. Polling for boot completion...");
        loop {
            let output = self
                .adb_command()
                .arg("shell")
                .arg("getprop")
                .arg("sys.boot_completed")
                .output()?;

            let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if output_str == "1" {
                println!("[ntest] Device boot completed.");
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        Ok(())
    }

    // Kept for compatibility if used elsewhere, but mainly delegates to
    // wait_for_device_online
    #[allow(dead_code)]
    fn wait_for_device(&self) -> anyhow::Result<()> {
        self.wait_for_device_online()
    }

    pub fn push_binary(&self) -> anyhow::Result<()> {
        let bin_path = if let Some(p) = &self.android_bin_path {
            p.clone()
        } else {
            println!("[ntest] WARNING: No android binary path provided. Assuming 'ntest' is already on device.");
            return Ok(());
        };

        println!("[ntest] Pushing binary from {} to /data/local/tmp/ntest", bin_path);
        let status =
            self.adb_command().arg("push").arg(&bin_path).arg("/data/local/tmp/ntest").status()?;

        if !status.success() {
            anyhow::bail!("Failed to push binary");
        }

        let _ = self.adb_command().arg("shell").arg("chmod +x /data/local/tmp/ntest").output()?;

        println!("[ntest] Binary pushed and chmod +x executed.");
        Ok(())
    }

    pub fn launch_netsim(&mut self) -> anyhow::Result<()> {
        if let Some(bin) = &self.netsim_bin {
            let mut cmd = Command::new(bin);
            if let Some(args) = &self.netsim_args {
                for arg in args.split_whitespace() {
                    cmd.arg(arg);
                }
            }
            println!("[ntest] Launching netsim: {:?}", cmd);
            let child = cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit()).spawn()?;
            self.netsim_process = Some(child);
            // Give netsim a moment to start
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        Ok(())
    }

    pub fn kill_emulator(&mut self) -> anyhow::Result<()> {
        if let Some(mut child) = self.emulator_process.take() {
            println!("[ntest] Killing emulator process...");
            let _ = child.kill();
            let _ = child.wait();
            println!("[ntest] Emulator killed.");
        }
        if let Some(mut child) = self.netsim_process.take() {
            println!("[ntest] Killing netsim process...");
            let _ = child.kill();
            let _ = child.wait();
            println!("[ntest] Netsim killed.");
        }
        Ok(())
    }

    pub fn wait_for_network(&self) -> anyhow::Result<()> {
        println!("[ntest] Waiting for network (ping 10.0.2.2)...");
        for _ in 0..30 {
            let status = self
                .adb_command()
                .arg("shell")
                .arg("ping -c 1 -W 1 10.0.2.2")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;

            if status.success() {
                println!("[ntest] Network is up.");
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        anyhow::bail!("Network unreachable after 30s")
    }
}

impl Drop for AndroidExecutor {
    fn drop(&mut self) {
        let _ = self.kill_emulator();
    }
}

impl Executor for AndroidExecutor {
    fn spawn_server(&self, port: u16) -> anyhow::Result<(Child, u16)> {
        spawn_server_common(port)
    }

    fn run_client(&self, params: ClientParams) -> anyhow::Result<std::process::Output> {
        let args = params.to_flags();
        println!(
            "[ntest executor] Running client: adb shell /data/local/tmp/ntest client {:?}",
            args
        );
        Ok(Command::new(&self.adb_path)
            .arg("shell")
            .arg("RUST_LOG=info")
            .arg("/data/local/tmp/ntest")
            .arg("client")
            .args(args)
            .output()?)
    }
}
