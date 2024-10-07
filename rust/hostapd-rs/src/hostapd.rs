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
use bytes::Bytes;
use log::warn;
use std::collections::HashMap;
use std::ffi::{c_char, c_int, CString};
use std::fs::File;
use std::io::{BufWriter, Write};
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

use crate::hostapd_sys::{run_hostapd_main, set_virtio_ctrl_sock, set_virtio_sock};

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
    _ctrl_writer: Option<Mutex<OwnedWriteHalf>>,
    tx_bytes: mpsc::Sender<Bytes>,
}

impl Hostapd {
    pub fn new(tx_bytes: mpsc::Sender<Bytes>, verbose: bool, config_path: PathBuf) -> Self {
        // Default Hostapd conf entries
        let config_data = vec![
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
        config.extend(config_data.into_iter().map(|(k, v)| (k.to_string(), v.to_string())));

        Hostapd {
            handle: RwLock::new(None),
            verbose,
            config,
            config_path,
            data_writer: None,
            _ctrl_writer: None,
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
        self._ctrl_writer = Some(Mutex::new(ctrl_writer));
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

    pub fn set_ssid(&mut self, _ssid: String, _password: String) -> bool {
        todo!();
    }

    pub fn get_ssid(&self) -> Option<String> {
        self.config.get("ssid").cloned()
    }

    /// Input data packet bytes from netsim to hostapd
    ///
    /// TODO:
    /// * update as async fn.
    pub fn input(&self, bytes: Bytes) -> anyhow::Result<()> {
        // Make sure hostapd is already running
        assert!(self.is_running(), "Failed to send input. Hostapd is not running.");
        Ok(get_runtime().block_on(async {
            let mut writer_guard = self.data_writer.as_ref().unwrap().lock().await;
            writer_guard.write_all(&bytes).await
        })?)
    }

    /// Check whether the hostapd thread is running
    pub fn is_running(&self) -> bool {
        let handle_lock = self.handle.read().unwrap();
        handle_lock.is_some() && !handle_lock.as_ref().unwrap().is_finished()
    }

    pub fn terminate(&self) {
        todo!();
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
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let stream = TcpStream::connect(addr).await?;
        let (listener, _) = listener.accept().await?;
        Ok((listener, stream))
    }

    /// Run the C hostapd process with run_hostapd_main
    ///
    /// This function is meant to be spawn in a separate thread.
    fn hostapd_thread(verbose: bool, config_path: String) {
        let mut args = vec![CString::new("hostapd").unwrap()];
        if verbose {
            args.push(CString::new("-dddd").unwrap())
        }
        args.push(
            CString::new(config_path.clone()).unwrap_or_else(|_| {
                panic!("CString::new error on config file path: {}", config_path)
            }),
        );
        let mut argv: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
        argv.push(std::ptr::null());
        let argc = argv.len() as c_int - 1;
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

            let size = match get_runtime().block_on(async { data_reader.read(&mut buf[..]).await })
            {
                Ok(size) => size,
                Err(e) => {
                    warn!("Failed to read hostapd response: {:?}", e);
                    break;
                }
            };

            if let Err(e) = tx_bytes.send(Bytes::from(buf[..size].to_vec())) {
                warn!("Failed to send hostapd packet response: {:?}", e);
                break;
            };
        }
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

/// Get or init the hostapd tokio runtime
/// TODO:
/// * make Runtime the responsibility of the caller.
fn get_runtime() -> &'static Arc<Runtime> {
    HOSTAPD_RUNTIME.get_or_init(|| Arc::new(Runtime::new().unwrap()))
}
