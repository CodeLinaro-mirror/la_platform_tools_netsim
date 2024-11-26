// Copyright 2024 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

///
/// This crate is a wrapper for hostapd C library.
///
/// Hostapd process is managed by a separate thread.
///
/// hostapd.conf file is generated under discovery directory.
///
use anyhow::bail;
use bytes::Bytes;
use log::{info, warn};
use std::collections::HashMap;
use std::ffi::{c_char, c_int, CStr, CString};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
#[cfg(unix)]
use std::os::fd::IntoRawFd;
#[cfg(windows)]
use std::os::windows::io::IntoRawSocket;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, OnceLock, RwLock};
use std::thread::{self, sleep};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpListener, TcpStream,
};
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

use crate::hostapd_sys::{
    run_hostapd_main, set_virtio_ctrl_sock, set_virtio_sock, VIRTIO_WIFI_CTRL_CMD_RELOAD_CONFIG,
    VIRTIO_WIFI_CTRL_CMD_TERMINATE,
};

/// Alias for RawFd on Unix or RawSocket on Windows (converted to i32)
type RawDescriptor = i32;

// TODO: Use a (global netsimd) tokio runtime from caller
static HOSTAPD_RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();

pub struct Hostapd {
    // TODO: update to tokio based RwLock when usages are async
    handle: RwLock<Option<thread::JoinHandle<()>>>,
    verbose: bool,
    config: HashMap<String, String>,
    config_path: PathBuf,
    data_writer: Option<Mutex<OwnedWriteHalf>>,
    ctrl_writer: Option<Mutex<OwnedWriteHalf>>,
    tx_bytes: mpsc::Sender<Bytes>,
}

impl Hostapd {
    pub fn new(tx_bytes: mpsc::Sender<Bytes>, verbose: bool, config_path: PathBuf) -> Self {
        // Default Hostapd conf entries
        let config_data = [
            ("ssid", "AndroidWifi"),
            ("interface", "wlan1"),
            ("driver", "virtio_wifi"),
            ("bssid", "00:13:10:95:fe:0b"),
            ("country_code", "US"),
            ("hw_mode", "g"),
            ("channel", "8"),
            ("beacon_int", "1000"),
            ("dtim_period", "2"),
            ("max_num_sta", "255"),
            ("rts_threshold", "2347"),
            ("fragm_threshold", "2346"),
            ("macaddr_acl", "0"),
            ("auth_algs", "3"),
            ("ignore_broadcast_ssid", "0"),
            ("wmm_enabled", "0"),
            ("ieee80211n", "1"),
            ("eapol_key_index_workaround", "0"),
        ];
        let mut config: HashMap<String, String> = HashMap::new();
        config.extend(config_data.iter().map(|(k, v)| (k.to_string(), v.to_string())));

        Hostapd {
            handle: RwLock::new(None),
            verbose,
            config,
            config_path,
            data_writer: None,
            ctrl_writer: None,
            tx_bytes,
        }
    }

    /// Start hostapd main process and pass responses to netsim
    /// The "hostapd" thread manages the C hostapd process by running "run_hostapd_main"
    /// The "hostapd_response" thread manages traffic between hostapd and netsim
    ///
    /// TODO:
    /// * update as async fn.
    pub fn run(&mut self) -> bool {
        // Check if already running
        assert!(!self.is_running(), "hostapd is already running!");
        // Setup config file
        self.gen_config_file().unwrap_or_else(|_| {
            panic!("Failed to generate config file: {:?}.", self.config_path.display())
        });

        // Setup Sockets
        let (ctrl_listener, _ctrl_reader, ctrl_writer) =
            Self::create_pipe().expect("Failed to create ctrl pipe");
        self.ctrl_writer = Some(Mutex::new(ctrl_writer));
        let (data_listener, data_reader, data_writer) =
            Self::create_pipe().expect("Failed to create data pipe");
        self.data_writer = Some(Mutex::new(data_writer));

        // Start hostapd thread
        let verbose = self.verbose;
        let config_path = self.config_path.to_string_lossy().into_owned();
        *self.handle.write().unwrap() = Some(
            thread::Builder::new()
                .name("hostapd".to_string())
                .spawn(move || Self::hostapd_thread(verbose, config_path))
                .expect("Failed to spawn Hostapd thread"),
        );

        // Start hostapd response thread
        let tx_bytes = self.tx_bytes.clone();
        let _ = thread::Builder::new()
            .name("hostapd_response".to_string())
            .spawn(move || {
                Self::hostapd_response_thread(data_listener, ctrl_listener, data_reader, tx_bytes);
            })
            .expect("Failed to spawn hostapd_response thread");

        true
    }

    /// Reconfigure Hostapd with specified SSID (and password) config
    ///
    /// TODO:
    /// * implement password & encryption support
    /// * update as async fn.
    pub fn set_ssid(
        &mut self,
        ssid: impl Into<String>,
        password: impl Into<String>,
    ) -> anyhow::Result<()> {
        let ssid = ssid.into();
        let password = password.into();
        if ssid.is_empty() {
            bail!("set_ssid must have a non-empty SSID");
        }

        if !password.is_empty() {
            bail!("set_ssid with password is not yet supported.");
        }

        if ssid == self.get_ssid() && password == self.get_config_val("password") {
            info!("SSID and password matches current configuration.");
            return Ok(());
        }

        // Update the config
        self.config.insert("ssid".to_string(), ssid);
        if !password.is_empty() {
            let password_config = [
                ("wpa", "2"),
                ("wpa_key_mgmt", "WPA-PSK"),
                ("rsn_pairwise", "CCMP"),
                ("wpa_passphrase", &password),
            ];
            self.config.extend(password_config.iter().map(|(k, v)| (k.to_string(), v.to_string())));
        }

        // Update the config file.
        self.gen_config_file()?;

        // Send command for Hostapd to reload config file
        if let Err(e) = get_runtime().block_on(Self::async_write(
            self.ctrl_writer.as_ref().unwrap(),
            c_string_to_bytes(VIRTIO_WIFI_CTRL_CMD_RELOAD_CONFIG),
        )) {
            bail!("Failed to send VIRTIO_WIFI_CTRL_CMD_RELOAD_CONFIG to hostapd to reload config: {:?}", e);
        }

        Ok(())
    }

    pub fn get_ssid(&self) -> String {
        self.get_config_val("ssid")
    }

    /// Input data packet bytes from netsim to hostapd
    ///
    /// TODO:
    /// * update as async fn.
    pub fn input(&self, bytes: Bytes) -> anyhow::Result<()> {
        // Make sure hostapd is already running
        assert!(self.is_running(), "Failed to send input. Hostapd is not running.");
        get_runtime().block_on(Self::async_write(self.data_writer.as_ref().unwrap(), &bytes))
    }

    /// Check whether the hostapd thread is running
    pub fn is_running(&self) -> bool {
        let handle_lock = self.handle.read().unwrap();
        handle_lock.is_some() && !handle_lock.as_ref().unwrap().is_finished()
    }

    pub fn terminate(&self) {
        if !self.is_running() {
            warn!("hostapd terminate() called when hostapd thread is not running");
            return;
        }

        // Send terminate command to hostapd
        if let Err(e) = get_runtime().block_on(Self::async_write(
            self.ctrl_writer.as_ref().unwrap(),
            c_string_to_bytes(VIRTIO_WIFI_CTRL_CMD_TERMINATE),
        )) {
            warn!("Failed to send VIRTIO_WIFI_CTRL_CMD_TERMINATE to hostapd to terminate: {:?}", e);
        }
    }

    /// Generate hostapd.conf in discovery directory
    fn gen_config_file(&self) -> anyhow::Result<()> {
        let conf_file = File::create(self.config_path.clone())?; // Create or overwrite the file
        let mut writer = BufWriter::new(conf_file);

        for (key, value) in &self.config {
            writeln!(&mut writer, "{}={}", key, value)?;
        }

        Ok(writer.flush()?) // Ensure all data is written to the file
    }

    /// Get the value of the given key in the config
    ///
    /// Returns empty String if key is not found
    fn get_config_val(&self, key: &str) -> String {
        self.config.get(key).cloned().unwrap_or_default()
    }

    /// Creates a pipe of two connected TcpStream objects
    ///
    /// Extracts the first stream's raw descriptor and splits the second stream as OwnedReadHalf and OwnedWriteHalf
    ///
    /// # Returns
    ///
    /// * `Ok((listener, read_half, write_half))` if the pipe creation is successful
    /// * `Err(std::io::Error)` if an error occurs during the pipe creation.
    fn create_pipe(
    ) -> anyhow::Result<(RawDescriptor, OwnedReadHalf, OwnedWriteHalf), std::io::Error> {
        let (listener, stream) = get_runtime().block_on(Self::async_create_pipe())?;
        let listener = into_raw_descriptor(listener);
        let (read_half, write_half) = stream.into_split();
        Ok((listener, read_half, write_half))
    }

    async fn async_create_pipe() -> anyhow::Result<(TcpStream, TcpStream), std::io::Error> {
        let listener = match TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await {
            Ok(listener) => listener,
            Err(e) => {
                // Support hosts that only have IPv6
                info!("Failed to bind to 127.0.0.1:0. Try to bind to [::1]:0 next. Err: {:?}", e);
                TcpListener::bind(SocketAddr::from((Ipv6Addr::LOCALHOST, 0))).await?
            }
        };
        let addr = listener.local_addr()?;
        let stream = TcpStream::connect(addr).await?;
        let (listener, _) = listener.accept().await?;
        Ok((listener, stream))
    }

    async fn async_write(writer: &Mutex<OwnedWriteHalf>, data: &[u8]) -> anyhow::Result<()> {
        let mut writer_guard = writer.lock().await;
        writer_guard.write_all(data).await?;
        writer_guard.flush().await?;
        Ok(())
    }

    /// Run the C hostapd process with run_hostapd_main
    ///
    /// This function is meant to be spawn in a separate thread.
    fn hostapd_thread(verbose: bool, config_path: String) {
        let mut args = vec![CString::new("hostapd").unwrap()];
        if verbose {
            args.push(CString::new("-dddd").unwrap());
        }
        args.push(
            CString::new(config_path.clone()).unwrap_or_else(|_| {
                panic!("CString::new error on config file path: {}", config_path)
            }),
        );
        let mut argv: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
        let argc = argv.len() as c_int;
        // Safety: we ensure that argc is length of argv and argv.as_ptr() is a valid pointer of hostapd args
        unsafe { run_hostapd_main(argc, argv.as_ptr()) };
    }

    /// Sets the virtio (driver) data and control sockets
    fn set_virtio_driver_socket(
        data_descriptor: RawDescriptor,
        ctrl_descriptor: RawDescriptor,
    ) -> bool {
        // Safety: we ensure that data_descriptor and ctrl_descriptor are valid i32 raw file descriptor or socket
        unsafe {
            set_virtio_sock(data_descriptor) == 0 && set_virtio_ctrl_sock(ctrl_descriptor) == 0
        }
    }

    /// Manage reading hostapd responses and sending via tx_bytes
    ///
    /// The thread first attempt to set virtio driver sockets with retries unitl success.
    /// Next the thread reads hostapd responses and writes to netsim
    fn hostapd_response_thread(
        data_listener: RawDescriptor,
        ctrl_listener: RawDescriptor,
        mut data_reader: OwnedReadHalf,
        tx_bytes: mpsc::Sender<Bytes>,
    ) {
        let mut buf: [u8; 1500] = [0u8; 1500];
        loop {
            if !Self::set_virtio_driver_socket(data_listener, ctrl_listener) {
                warn!("Unable to set virtio driver socket. Retrying...");
                sleep(Duration::from_millis(250));
                continue;
            };
            break;
        }
        loop {
            let size = match get_runtime().block_on(async { data_reader.read(&mut buf[..]).await })
            {
                Ok(size) => size,
                Err(e) => {
                    warn!("Failed to read hostapd response: {:?}", e);
                    break;
                }
            };

            if let Err(e) = tx_bytes.send(Bytes::copy_from_slice(&buf[..size])) {
                warn!("Failed to send hostapd packet response: {:?}", e);
                break;
            };
        }
    }
}

impl Drop for Hostapd {
    fn drop(&mut self) {
        self.terminate();
    }
}

/// Convert TcpStream to RawDescriptor (i32)
fn into_raw_descriptor(stream: TcpStream) -> RawDescriptor {
    let std_stream = stream.into_std().expect("into_raw_descriptor's into_std() failed");
    // hostapd fd expects blocking, but rust set non-blocking for async
    std_stream.set_nonblocking(false).expect("non-blocking");

    // Use into_raw_fd for Unix to pass raw file descriptor to C
    #[cfg(unix)]
    return std_stream.into_raw_fd();

    // Use into_raw_socket for Windows to pass raw socket to C
    #[cfg(windows)]
    std_stream.into_raw_socket().try_into().expect("Failed to convert Raw Socket value into i32")
}

/// Convert a null terminated c-string slice into &[u8] bytes without the nul terminator
fn c_string_to_bytes(c_string: &[u8]) -> &[u8] {
    CStr::from_bytes_with_nul(c_string).unwrap().to_bytes()
}

/// Get or init the hostapd tokio runtime
/// TODO:
/// * make Runtime the responsibility of the caller.
fn get_runtime() -> &'static Arc<Runtime> {
    HOSTAPD_RUNTIME.get_or_init(|| Arc::new(Runtime::new().unwrap()))
}
