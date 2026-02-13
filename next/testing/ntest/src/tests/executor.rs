use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    process::{Child, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};

use anyhow::Context;
use async_trait::async_trait;

pub const LABEL_WIDTH: usize = 12;

// Protocol markers for Guest-Host communication
const MARKER_RECEIVED: &str = ">> RECEIVED:";
const MARKER_COMPLETED: &str = "<< COMPLETED:";
const MARKER_FATAL: &str = "!! ERROR:";
const RESULT_SUCCESS: &str = "RESULT=SUCCESS";
const RESULT_FAILURE: &str = "RESULT=FAILURE";
const MSG_KEY: &str = "MSG=";

/// Throughput data collected during a client run.
#[derive(Debug, Clone)]
pub struct Throughput {
    pub bytes: usize,
    pub duration: Duration,
}

/// Interface for executing commands on a test actor (Host or Guest).
#[async_trait]
pub trait Executor: Send + Sync {
    /// Execute a network client task and return throughput if successful.
    async fn run_client(&self, params: ClientParams) -> anyhow::Result<Option<Throughput>>;
    /// Execute an arbitrary BDD step on the actor.
    async fn execute_step(&self, step: &str) -> anyhow::Result<()>;
    /// Restart the agent process on the target.
    async fn reset_agent(&mut self) -> anyhow::Result<()>;
    /// Get the display label for this executor (e.g., "@Small_Phone").
    fn get_label(&self) -> String;
    /// Get the unique serial number if available.
    fn get_serial(&self) -> Option<String>;
    /// Enable or disable live feedback logging from this executor.
    fn set_silent(&self, silent: bool);
}

/// Parameters for a network client execution.
pub struct ClientParams {
    pub payload_size: usize,
    pub proto: String,
    pub target: String,
    pub expect_eof: bool,
    pub timeout_ms: u64,
}

/// Executor for Android Guest devices using ADB.
pub struct AndroidExecutor {
    pub adb_path: String,
    pub serial: Option<String>,
    pub agent_process: Option<Child>,
    pub netsim_process: Option<Child>,
    pub apk_path: Option<String>,
    pub avd_name: Option<String>,
    pub feedback_port: Option<u16>,
    pub feedback_stream: Arc<Mutex<Option<TcpStream>>>,
    pub feedback_receiver: Arc<Mutex<Option<mpsc::Receiver<String>>>>,
    pub silent: Arc<AtomicBool>,
}

impl AndroidExecutor {
    /// Find adb binary in standard locations or provided path.
    pub fn find_adb(provided_path: Option<&str>) -> String {
        if let Some(path) = provided_path {
            let p = std::path::Path::new(path);
            if p.is_file() {
                return path.to_string();
            }
            // Check provided path directly
            let adb = p.join("adb");
            if adb.exists() {
                return adb.to_string_lossy().to_string();
            }
            // Check platform-tools subdirectory (if it's ANDROID_HOME)
            let adb = p.join("platform-tools").join("adb");
            if adb.exists() {
                return adb.to_string_lossy().to_string();
            }
        }
        if let Ok(path) = which::which("adb") {
            return path.to_string_lossy().to_string();
        }
        for var in &["ANDROID_HOME", "ANDROID_SDK_ROOT", "ANDROID_SDK_HOME"] {
            if let Ok(val) = std::env::var(var) {
                let p = std::path::Path::new(&val).join("platform-tools").join("adb");
                if p.exists() {
                    return p.to_string_lossy().to_string();
                }
            }
        }
        "adb".to_string()
    }

    pub fn new(
        serial: Option<String>,
        android_home: String,
        apk_path: Option<String>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            adb_path: android_home,
            apk_path,
            serial,
            agent_process: None,
            netsim_process: None,
            avd_name: None,
            feedback_port: None,
            feedback_stream: Arc::new(Mutex::new(None)),
            feedback_receiver: Arc::new(Mutex::new(None)),
            silent: Arc::new(AtomicBool::new(false)),
        })
    }

    fn adb_command(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new(&self.adb_path);
        if let Some(s) = &self.serial {
            cmd.arg("-s").arg(s);
        }
        cmd
    }

    pub fn launch_netsim(
        &mut self,
        bin: &Option<String>,
        args: &Option<String>,
    ) -> anyhow::Result<()> {
        if let Some(bin_path) = bin {
            let mut cmd = std::process::Command::new(bin_path);
            if let Some(a) = args {
                for arg in a.split_whitespace() {
                    cmd.arg(arg);
                }
            }
            self.netsim_process = Some(cmd.stdout(Stdio::null()).stderr(Stdio::null()).spawn()?);
        }
        Ok(())
    }

    pub fn install_apk(&mut self) -> anyhow::Result<()> {
        if let Some(apk) = &self.apk_path {
            let status = self
                .adb_command()
                .arg("install")
                .arg("-r")
                .arg(apk)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
            if !status.success() {
                anyhow::bail!("Failed to install APK: {}", apk);
            }
        }
        Ok(())
    }

    /// Set up TCP reverse tunneling for feedback from the Kotlin agent.
    pub fn setup_feedback(&mut self) -> anyhow::Result<()> {
        let listener = TcpListener::bind("0.0.0.0:0")?;
        let port = listener.local_addr()?.port();
        self.feedback_port = Some(port);
        self.adb_command()
            .arg("reverse")
            .arg(format!("tcp:{}", port))
            .arg(format!("tcp:{}", port))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        let name = self.get_label();
        let feedback_stream = self.feedback_stream.clone();
        let silent = self.silent.clone();
        let (tx, rx) = mpsc::channel();
        {
            let mut guard = self.feedback_receiver.lock().unwrap();
            *guard = Some(rx);
        }

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(s) => {
                        let name_clone = name.clone();
                        if let Ok(cloned) = s.try_clone() {
                            let mut guard = feedback_stream.lock().unwrap();
                            *guard = Some(cloned);
                        }
                        let tx_clone = tx.clone();
                        let silent_clone = silent.clone();
                        std::thread::spawn(move || {
                            let reader = BufReader::new(s);
                            for line in reader.lines().map_while(Result::ok) {
                                // Filter out protocol markers from the narrative output
                                if !line.contains(MARKER_RECEIVED)
                                    && !line.contains(MARKER_COMPLETED)
                                {
                                    if !silent_clone.load(Ordering::SeqCst) {
                                        Self::print_bdd_line(&name_clone, &line);
                                    }
                                }
                                let _ = tx_clone.send(line);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("!! ERR: Failed to accept feedback connection: {}", e);
                    }
                }
            }
        });
        Ok(())
    }

    fn print_bdd_line(name: &str, line: &str) {
        let line = line.trim();
        let actor_tag = format!("@{:width$}", name, width = LABEL_WIDTH);
        if line.starts_with("GIVEN")
            || line.starts_with("WHEN")
            || line.starts_with("AND")
            || line.starts_with("THEN")
        {
            let mut parts = line.splitn(2, ' ');
            let verb = parts.next().unwrap_or("");
            let msg = parts.next().unwrap_or("");
            println!("{:<6} {} {}", verb, actor_tag, msg);
        } else if let Some(msg) = line.strip_prefix("INFO") {
            let msg = msg.trim();
            if !msg.starts_with('[') {
                println!("{:<6} {} {}", "INFO", actor_tag, msg);
            }
        }
        let _ = std::io::stdout().flush();
    }

    pub fn launch_agent(&mut self) -> anyhow::Result<()> {
        let name = self.get_label();
        let port_arg =
            self.feedback_port.ok_or_else(|| anyhow::anyhow!("Feedback port not set"))?.to_string();
        self.agent_process = Some(
            self.adb_command()
                .arg("shell")
                .arg("am")
                .arg("instrument")
                .arg("-w")
                .arg("-e")
                .arg("wait")
                .arg("true")
                .arg("-e")
                .arg("device_name")
                .arg(&name)
                .arg("-e")
                .arg("control_port")
                .arg(&port_arg)
                .arg("com.android.netsim.ntest/.NTestInstrumentation")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?,
        );
        Ok(())
    }

    pub fn wait_for_feedback(&self) -> anyhow::Result<()> {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(60) {
            if self.feedback_stream.lock().unwrap().is_some() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        anyhow::bail!("Feedback timeout waiting for Kotlin agent to connect (60s)")
    }

    /// Blocks until a specific pattern is received on the feedback channel.
    /// This is the primary synchronization primitive for Guest orchestration.
    async fn wait_for_pattern(&self, pattern: &str, timeout_secs: u64) -> anyhow::Result<String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let pattern = pattern.to_string();
        let receiver = self.feedback_receiver.clone();

        std::thread::spawn(move || {
            let rx_guard = receiver.lock().unwrap();
            if let Some(receiver_rx) = rx_guard.as_ref() {
                let start = Instant::now();
                while start.elapsed().as_secs() < timeout_secs {
                    // Poll with a small sleep to avoid pegged CPU while maintaining responsiveness
                    if let Ok(line) = receiver_rx.recv_timeout(Duration::from_millis(10)) {
                        if line.contains(&pattern) {
                            let _ = tx.send(Ok(line));
                            return;
                        }
                        if line.contains(MARKER_FATAL) {
                            let _ = tx.send(Err(anyhow::anyhow!("Guest fatal error: {}", line)));
                            return;
                        }
                    }
                }
                let _ = tx.send(Err(anyhow::anyhow!("Timeout waiting for pattern: {}", pattern)));
            }
        });

        tokio::time::timeout(Duration::from_secs(timeout_secs + 1), rx)
            .await
            .context("Pattern wait timed out at the orchestrator level")??
    }
}

#[async_trait]
impl Executor for AndroidExecutor {
    async fn run_client(&self, params: ClientParams) -> anyhow::Result<Option<Throughput>> {
        let step = format!(
            "When Android sends {} bytes of {} data to {}",
            params.payload_size,
            params.proto.to_uppercase(),
            params.target
        );

        let start = Instant::now();
        self.execute_step(&step).await?;
        let duration = start.elapsed();

        Ok(Some(Throughput { bytes: params.payload_size, duration }))
    }

    async fn execute_step(&self, step: &str) -> anyhow::Result<()> {
        {
            let mut guard = self.feedback_stream.lock().unwrap();
            let stream =
                guard.as_mut().ok_or_else(|| anyhow::anyhow!("Feedback channel disconnected"))?;
            writeln!(stream, "{}", step)?;
            stream.flush()?;
        }

        // 1. Wait for acknowledgement that the agent received the command
        self.wait_for_pattern(&format!("{} {}", MARKER_RECEIVED, step), 5).await?;

        // 2. Wait for the completion signal (with a generous 60s timeout for network
        //    tasks)
        let complete_line =
            self.wait_for_pattern(&format!("{} {}", MARKER_COMPLETED, step), 60).await?;

        if complete_line.contains(RESULT_FAILURE) {
            let msg = complete_line.split(MSG_KEY).last().unwrap_or("Unknown actor-side error");
            anyhow::bail!("Step failed on actor: {}", msg);
        }
        Ok(())
    }

    async fn reset_agent(&mut self) -> anyhow::Result<()> {
        let quit_step = "THEN Android Quits";
        {
            let mut guard = self.feedback_stream.lock().unwrap();
            if let Some(stream) = guard.as_mut() {
                let _ = writeln!(stream, "{}", quit_step);
                let _ = stream.flush();
                *guard = None;
            }
        }
        let quit_pattern = format!("{} {}", MARKER_RECEIVED, quit_step);
        let _ = self.wait_for_pattern(&quit_pattern, 2).await;
        if let Some(mut child) = self.agent_process.take() {
            let _ = child.wait();
        }
        std::thread::sleep(Duration::from_millis(100));
        self.launch_agent()?;
        self.wait_for_feedback()
    }

    fn get_label(&self) -> String {
        self.avd_name.clone().or(self.serial.clone()).unwrap_or_else(|| "Android".to_string())
    }

    fn get_serial(&self) -> Option<String> {
        self.serial.clone()
    }

    fn set_silent(&self, silent: bool) {
        self.silent.store(silent, Ordering::SeqCst);
    }
}
