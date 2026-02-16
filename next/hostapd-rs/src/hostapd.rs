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

//! Controller interface for the `hostapd` C library.
//!
//! This module allows interaction with `hostapd` to manage WiFi access point
//! and various wireless networking tasks directly from Rust code.
//!
//! The main `hostapd` process is managed by a separate task while responses
//! from the `hostapd` process are handled by another task, ensuring efficient
//! and non-blocking communication.
//!
//! `hostapd` configuration consists of key-value pairs. The default
//! configuration file is generated in the discovery directory.
//!
//! ## Features
//!
//! * **Asynchronous operation:** The module utilizes `tokio` for asynchronous
//!   communication with the `hostapd` process, allowing for efficient and
//!   non-blocking operations.
//! * **Platform support:** Supports Linux, macOS, and Windows.
//! * **Configuration management:** Provides functionality to generate and
//!   manage `hostapd` configuration files.
//! * **Easy integration:** Offers a high-level API to simplify interaction with
//!   `hostapd`, abstracting away low-level details.
//!
//! ## Usage
//!
//! Here's a basic example of how to create a `Hostapd` instance and start the
//! `hostapd` process:
//!
//! ```
//! use std::path::PathBuf;
//!
//! use hostapd_rs::hostapd::Hostapd;
//! use tokio::{runtime::Runtime, sync::mpsc};
//!
//! let rt = Runtime::new().unwrap();
//! rt.block_on(async {
//!     // Create a channel for receiving data from hostapd
//!     let (tx, _) = mpsc::channel(100);
//!
//!     // Create a new Hostapd instance
//!     let mut hostapd = Hostapd::new(
//!         tx,                                 // Sender for receiving data
//!         false,                              // Verbose mode
//!         PathBuf::from("/tmp/hostapd.conf"), // Path to the configuration file
//!     );
//!
//!     // Start the hostapd process
//!     hostapd.run().await;
//! });
//! ```
//!
//! This starts `hostapd` in a separate task, allowing interaction with it using
//! the `Hostapd` struct's methods.

#[cfg(unix)]
use std::os::fd::IntoRawFd;
#[cfg(windows)]
use std::os::windows::io::IntoRawSocket;
use std::{
    collections::HashMap,
    ffi::{c_char, c_int, CStr, CString},
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    path::PathBuf,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc, Mutex as StdMutex, RwLock,
    },
    time::Duration,
};

use aes::Aes128;
use anyhow::bail;
use bytes::Bytes;
use ccm::{
    aead::{generic_array::GenericArray, Aead, Payload},
    consts::{U13, U8},
    Ccm, KeyInit,
};
use log::{debug, info, warn};
use netsim_packets::ieee80211::{parse_mac_address, Ieee80211, MacAddress, CCMP_HDR_LEN};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener, TcpStream,
    },
    sync::{mpsc, Mutex, RwLock as AsyncRwLock},
    task::JoinHandle,
    time::sleep,
};

#[cfg(not(test))]
use crate::hostapd_sys::{get_active_gtk, get_active_ptk};
use crate::hostapd_sys::{
    run_hostapd_main, set_virtio_ctrl_sock, set_virtio_sock, VIRTIO_WIFI_CTRL_CMD_RELOAD_CONFIG,
    VIRTIO_WIFI_CTRL_CMD_TERMINATE,
};

fn c_string_to_bytes(c_string: &[u8]) -> &[u8] {
    CStr::from_bytes_with_nul(c_string).expect("c_string_to_bytes error").to_bytes()
}

/// Alias for RawFd on Unix or RawSocket on Windows (converted to i32)
type RawDescriptor = i32;
type KeyData = [u8; 32];

/// Hostapd process interface.
///
/// This struct provides methods for interacting with the `hostapd` process,
/// such as starting and stopping the process, configuring the access point,
/// and sending and receiving data.
pub struct Hostapd {
    task_handle: AsyncRwLock<Option<JoinHandle<()>>>,
    verbose: bool,
    config: RwLock<HashMap<String, String>>,
    config_path: PathBuf,
    data_writer: StdMutex<Option<Arc<Mutex<OwnedWriteHalf>>>>,
    ctrl_writer: StdMutex<Option<Arc<Mutex<OwnedWriteHalf>>>>,
    tx_bytes: mpsc::Sender<Bytes>,
    // MAC address of the access point.
    bssid: RwLock<MacAddress>,
    // Current transmit packet number (PN) used for encryption
    tx_pn: AtomicI64,
}

impl Hostapd {
    /// Creates a new `Hostapd` instance.
    ///
    /// # Arguments
    ///
    /// * `tx_bytes`: Sender for transmitting data received from `hostapd`.
    /// * `verbose`: Whether to run `hostapd` in verbose mode.
    /// * `config_path`: Path to the `hostapd` configuration file.

    pub fn new(tx_bytes: mpsc::Sender<Bytes>, verbose: bool, config_path: PathBuf) -> Self {
        // Default Hostapd conf entries
        let bssid = "00:13:10:85:fe:01";
        let config_data = [
            ("ssid", "AndroidWifi"),
            ("interface", "wlan1"),
            ("driver", "virtio_wifi"),
            ("bssid", bssid),
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
            task_handle: AsyncRwLock::new(None),
            verbose,
            config: RwLock::new(config),
            config_path,
            data_writer: StdMutex::new(None),
            ctrl_writer: StdMutex::new(None),
            tx_bytes,
            bssid: RwLock::new(parse_mac_address(bssid).unwrap()),
            tx_pn: AtomicI64::new(1),
        }
    }

    /// Starts the `hostapd` main process and response task.
    ///
    /// The "hostapd" task manages the C `hostapd` process by running
    /// `run_hostapd_main`. The "hostapd_response" task manages traffic
    /// between `hostapd` and netsim.
    pub async fn run(&self) -> bool {
        debug!("Running hostapd with config: {:?}", &self.config.read().expect("Poisoned lock"));

        // Check if already running
        if self.is_running().await {
            panic!("hostapd is already running!");
        }
        // Setup config file
        if let Err(e) = self.gen_config_file().await {
            panic!(
                "Failed to generate config file: {:?}. Error: {:?}",
                self.config_path.display(),
                e
            );
        }

        // Setup Sockets
        let (ctrl_listener, _ctrl_reader, ctrl_writer) =
            self.create_pipe().await.expect("Failed to create ctrl pipe");
        *self.ctrl_writer.lock().expect("Poisoned lock") = Some(Arc::new(Mutex::new(ctrl_writer)));
        let (data_listener, data_reader, data_writer) =
            self.create_pipe().await.expect("Failed to create data pipe");
        *self.data_writer.lock().expect("Poisoned lock") = Some(Arc::new(Mutex::new(data_writer)));

        // Start hostapd task
        let verbose = self.verbose;
        let config_path = self.config_path.to_string_lossy().into_owned();
        let task_handle = tokio::task::spawn_blocking(move || {
            Self::hostapd_task(verbose, config_path);
        });
        *self.task_handle.write().await = Some(task_handle);

        // Start hostapd response task
        let tx_bytes = self.tx_bytes.clone();
        let _response_handle = tokio::spawn(async move {
            Self::hostapd_response_task(data_listener, ctrl_listener, data_reader, tx_bytes).await;
        });
        // We don't need to store response_handle as we don't need to explicitly manage
        // it after start.

        true
    }

    /// Reconfigures `Hostapd` with the specified SSID (and password).
    pub async fn set_ssid(
        &self,
        ssid: impl Into<String>,
        password: impl Into<String>,
    ) -> anyhow::Result<()> {
        let ssid = ssid.into();
        let password = password.into();
        if ssid.is_empty() {
            bail!("set_ssid must have a non-empty SSID");
        }

        if ssid == self.get_ssid() && password == self.get_config_val("wpa_passphrase") {
            debug!("SSID and password matches current configuration.");
            return Ok(());
        }

        // Update the config
        {
            let mut config = self.config.write().expect("Poisoned lock");
            config.insert("ssid".to_string(), ssid);
        }
        if !password.is_empty() {
            let password_config = [
                ("wpa", "2"),
                ("wpa_key_mgmt", "WPA-PSK"),
                ("rsn_pairwise", "CCMP"),
                ("wpa_passphrase", &password),
            ];
            {
                let mut config = self.config.write().expect("Poisoned lock");
                config.extend(password_config.iter().map(|(k, v)| (k.to_string(), v.to_string())));
            }
        }

        // Update the config file.
        self.gen_config_file().await?;

        // Send command for Hostapd to reload config file
        if self.is_running().await {
            let cmd = c_string_to_bytes(VIRTIO_WIFI_CTRL_CMD_RELOAD_CONFIG);
            let ctrl_writer = self.ctrl_writer.lock().expect("Poisoned lock").clone();
            if let Some(writer) = ctrl_writer {
                if let Err(e) = Self::async_write(&writer, cmd).await {
                    bail!("Failed to send VIRTIO_WIFI_CTRL_CMD_RELOAD_CONFIG to hostapd to reload config: {:?}", e);
                }
            } else {
                bail!("Hostapd failed to reload config: ctrl_writer missing");
            }
        }

        Ok(())
    }

    /// Reconfigures `Hostapd` with the specified BSSID.
    pub async fn set_bssid(&self, bssid: &MacAddress) -> anyhow::Result<()> {
        if *bssid == self.get_bssid() {
            return Ok(());
        }
        *self.bssid.write().expect("Poisoned lock") = *bssid;
        self.config.write().expect("Poisoned lock").insert("bssid".to_string(), bssid.to_string());

        // Update the config file.
        self.gen_config_file().await?;

        // Send command for Hostapd to reload config file
        if self.is_running().await {
            let cmd = c_string_to_bytes(VIRTIO_WIFI_CTRL_CMD_RELOAD_CONFIG);
            let ctrl_writer = self.ctrl_writer.lock().expect("Poisoned lock").clone();
            if let Some(writer) = ctrl_writer {
                if let Err(e) = Self::async_write(&writer, cmd).await {
                    bail!("Failed to send VIRTIO_WIFI_CTRL_CMD_RELOAD_CONFIG to hostapd to reload config: {:?}", e);
                }
            } else {
                bail!("Hostapd failed to reload config: ctrl_writer missing");
            }
        }
        Ok(())
    }

    /// Retrieves the current SSID in the `Hostapd` configuration.
    pub fn get_ssid(&self) -> String {
        self.get_config_val("ssid")
    }

    /// Retrieves the `Hostapd`'s BSSID.
    pub fn get_bssid(&self) -> MacAddress {
        *self.bssid.read().expect("Poisoned lock")
    }

    /// Generate the next packet number
    pub fn gen_packet_number(&self) -> [u8; 6] {
        let tx_pn = self.tx_pn.fetch_add(1, Ordering::Relaxed);
        tx_pn.to_be_bytes()[2..].try_into().unwrap()
    }

    /// Retrieve the current active GTK or PTK key data from Hostapd
    #[cfg(not(test))]
    fn get_key(&self, ieee80211: &Ieee80211) -> (KeyData, usize, u8) {
        // Determine key (GTK for multicast/broadcast, PTK for unicast)
        let key = if ieee80211.is_multicast() || ieee80211.is_broadcast() {
            // SAFETY: get_active_gtk requires no input and returns a virtio_wifi_key_data
            // struct
            unsafe { get_active_gtk() }
        } else {
            // SAFETY: get_active_ptk requires no input and returns a virtio_wifi_key_data
            // struct
            unsafe { get_active_ptk() }
        };

        // Return key data, length, and index from virtio_wifi_key_data
        (key.key_material, key.key_len as usize, key.key_idx as u8)
    }

    /// Attempt to encrypt the given IEEE 802.11 frame.
    pub fn try_encrypt(&self, ieee80211: &Ieee80211) -> Option<Vec<u8>> {
        // Check if encryption is needed
        if !ieee80211.needs_encryption() {
            return None;
        }

        // Retrieve current active key & skip encryption if key is not available
        let (key_material, key_len, key_id) = self.get_key(ieee80211);
        if key_len == 0 {
            return None;
        }
        let key = GenericArray::from_slice(&key_material[..key_len]);

        // Prep encryption parameters
        let cipher = Ccm::<Aes128, U8, U13>::new(key);
        let pn = self.gen_packet_number();
        let nonce_binding = &ieee80211.get_nonce(&pn);
        let nonce = GenericArray::from_slice(nonce_binding);

        // Encrypt the data with nonce and aad
        let payload = ieee80211.get_payload();
        let ciphertext =
            match cipher.encrypt(nonce, Payload { msg: &payload, aad: &ieee80211.get_aad() }) {
                Ok(ciphertext) => ciphertext,
                Err(e) => {
                    warn!("Encryption error: {:?}", e);
                    return None;
                }
            };

        // We want to construct:
        // Header (up to hdr_length) + CCMP Header (8 bytes) + Ciphertext.
        let mut new_packet =
            Vec::with_capacity(ieee80211.hdr_length() + CCMP_HDR_LEN + ciphertext.len());
        new_packet.extend_from_slice(&ieee80211.as_bytes()[..ieee80211.hdr_length()]);

        // CCMP Header
        new_packet.extend_from_slice(&[
            pn[5],
            pn[4],
            0,                    // Reserved
            0x20 | (key_id << 6), // Key ID + Ext IV
            pn[3],
            pn[2],
            pn[1],
            pn[0],
        ]);

        new_packet.extend_from_slice(&ciphertext);

        // Set protected bit in Frame Control (byte 1, bit 6 i.e. 0x40)
        // Frame Control is at offset 0.
        new_packet[1] |= 0x40;

        Some(new_packet)
    }

    /// Attempt to decrypt the given IEEE 802.11 frame.
    pub fn try_decrypt(&self, ieee80211: &Ieee80211) -> Option<Vec<u8>> {
        if !ieee80211.needs_decryption() {
            return None;
        }

        // Retrieve current active key, skip decryption if key is not available
        let (key_material, key_len, _) = self.get_key(ieee80211);
        if key_len == 0 {
            return None;
        }
        let key = GenericArray::from_slice(&key_material[..key_len]);

        // Prep encryption parameters
        let cipher = Ccm::<Aes128, U8, U13>::new(key);
        let pn = &ieee80211.get_packet_number();
        let nonce_binding = &ieee80211.get_nonce(pn);
        let nonce = GenericArray::from_slice(nonce_binding);

        // Extract data (Ciphertext)
        // Data starts after Header + CCMP Header.
        let hdr_len = ieee80211.hdr_length();
        let data_offset = hdr_len + CCMP_HDR_LEN;
        if ieee80211.as_bytes().len() < data_offset {
            return None;
        }
        let data = &ieee80211.as_bytes()[data_offset..];
        let aad = ieee80211.get_aad();

        // Decrypt the data

        let plaintext = match cipher.decrypt(nonce, Payload { msg: data, aad: &aad }) {
            Ok(plaintext) => plaintext,
            Err(e) => {
                warn!("Decryption error: {:?}", e);
                return None;
            }
        };

        // Construct the decrypted frame
        // Header + Plaintext
        let mut new_packet = Vec::with_capacity(hdr_len + plaintext.len());
        new_packet.extend_from_slice(&ieee80211.as_bytes()[..hdr_len]);
        new_packet.extend_from_slice(&plaintext);

        // Reset protected bit
        new_packet[1] &= !0x40;

        Some(new_packet)
    }

    /// Inputs data packet bytes from netsim to `hostapd`.
    pub async fn input(&self, bytes: Bytes) -> anyhow::Result<()> {
        // Make sure hostapd is already running
        if !self.is_running().await {
            panic!("Failed to send input. Hostapd is not running.");
        }
        let data_writer = self.data_writer.lock().expect("Poisoned lock").clone();
        if let Some(writer) = data_writer {
            Self::async_write(&writer, &bytes).await
        } else {
            panic!("Failed to send input. data_writer missing.");
        }
    }

    /// Checks whether the `hostapd` task is running.
    pub async fn is_running(&self) -> bool {
        let task_handle_lock = self.task_handle.read().await;
        task_handle_lock.is_some() && !task_handle_lock.as_ref().unwrap().is_finished()
    }

    /// Terminates the `Hostapd` process task by sending a control command.
    pub async fn terminate(&self) {
        if !self.is_running().await {
            warn!("hostapd terminate() called when hostapd task is not running");
            return;
        }

        let ctrl_writer = self.ctrl_writer.lock().expect("Poisoned lock").clone();
        if let Some(writer) = ctrl_writer {
            if let Err(e) =
                Self::async_write(&writer, c_string_to_bytes(VIRTIO_WIFI_CTRL_CMD_TERMINATE)).await
            {
                warn!(
                    "Failed to send VIRTIO_WIFI_CTRL_CMD_TERMINATE to hostapd to terminate: {:?}",
                    e
                );
            }
        } else {
            warn!("Failed to terminate hostapd: ctrl_writer missing");
        }
        // Wait for hostapd task to finish.
        if let Some(task_handle) = self.task_handle.write().await.take() {
            if let Err(e) = task_handle.await {
                warn!("Failed to join hostapd task during terminate: {:?}", e);
            }
        }
    }

    /// Generates the `hostapd.conf` file in the discovery directory.
    async fn gen_config_file(&self) -> anyhow::Result<()> {
        let conf_file = File::create(self.config_path.clone()).await?; // Create or overwrite the file
        let mut writer = BufWriter::new(conf_file);

        let config: Vec<(String, String)> = self
            .config
            .read()
            .expect("Poisoned lock")
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (key, value) in config {
            let line = format!("{}={}\n", key, value);
            writer.write_all(line.as_bytes()).await?;
        }

        writer.flush().await?; // Ensure all data is written to the file
        Ok(())
    }

    /// Gets the value of the given key in the config.
    ///
    /// Returns an empty String if the key is not found.
    /// Gets the value of the given key in the config.
    ///
    /// Returns an empty String if the key is not found.
    fn get_config_val(&self, key: &str) -> String {
        self.config.read().expect("Poisoned lock").get(key).cloned().unwrap_or_default()
    }

    /// Creates a pipe of two connected `TcpStream` objects.
    ///
    /// Extracts the first stream's raw descriptor and splits the second stream
    /// into `OwnedReadHalf` and `OwnedWriteHalf`.
    ///
    /// # Returns
    ///
    /// * `Ok((listener, read_half, write_half))` if the pipe creation is
    ///   successful.
    /// * `Err(std::io::Error)` if an error occurs during pipe creation.
    async fn create_pipe(
        &self,
    ) -> anyhow::Result<(RawDescriptor, OwnedReadHalf, OwnedWriteHalf), std::io::Error> {
        let (listener_stream, stream) = Self::async_create_pipe().await?;
        let listener = into_raw_descriptor(listener_stream);
        let (read_half, write_half) = stream.into_split();
        Ok((listener, read_half, write_half))
    }

    /// Creates a pipe asynchronously.
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
        let (listener_stream, _) = listener.accept().await?;
        Ok((listener_stream, stream))
    }

    /// Writes data to a writer asynchronously.
    async fn async_write(writer: &Mutex<OwnedWriteHalf>, data: &[u8]) -> anyhow::Result<()> {
        let mut writer_guard = writer.lock().await;
        writer_guard.write_all(data).await?;
        writer_guard.flush().await?;
        Ok(())
    }

    /// Runs the C `hostapd` process with `run_hostapd_main`.
    ///
    /// This function is meant to be spawned in a separate task.
    fn hostapd_task(verbose: bool, config_path: String) {
        let mut args = vec![CString::new("hostapd").unwrap()];
        if verbose {
            args.push(CString::new("-dddd").unwrap());
        }
        args.push(
            CString::new(config_path.clone()).unwrap_or_else(|_| {
                panic!("CString::new error on config file path: {}", config_path)
            }),
        );
        let argv: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
        let argc = argv.len() as c_int;
        // Safety: we ensure that argc is length of argv and argv.as_ptr() is a valid
        // pointer of hostapd args
        unsafe { run_hostapd_main(argc, argv.as_ptr()) };
    }

    /// Sets the virtio (driver) data and control sockets.
    fn set_virtio_driver_socket(
        data_descriptor: RawDescriptor,
        ctrl_descriptor: RawDescriptor,
    ) -> bool {
        // Safety: we ensure that data_descriptor and ctrl_descriptor are valid i32 raw
        // file descriptor or socket
        unsafe {
            set_virtio_sock(data_descriptor) == 0 && set_virtio_ctrl_sock(ctrl_descriptor) == 0
        }
    }

    /// Manages reading `hostapd` responses and sending them via `tx_bytes`.
    ///
    /// The task first attempts to set virtio driver sockets with retries until
    /// success. Next, the task reads `hostapd` responses and writes them to
    /// netsim.
    async fn hostapd_response_task(
        data_descriptor: RawDescriptor,
        ctrl_descriptor: RawDescriptor,
        mut data_reader: OwnedReadHalf,
        tx_bytes: mpsc::Sender<Bytes>,
    ) {
        let mut buf: [u8; 1500] = [0u8; 1500];
        loop {
            if !Self::set_virtio_driver_socket(data_descriptor, ctrl_descriptor) {
                warn!("Unable to set virtio driver socket. Retrying...");
                sleep(Duration::from_millis(250)).await;
                continue;
            };
            break;
        }
        loop {
            let size = match data_reader.read(&mut buf[..]).await {
                Ok(size) => size,
                Err(_e) => {
                    break;
                }
            };

            if let Err(_e) = tx_bytes.send(Bytes::copy_from_slice(&buf[..size])).await {
                break;
            };
        }
    }
}

/// Converts a `TcpStream` to a `RawDescriptor` (i32).
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

// Removed c_string_to_bytes as constants are now raw bytes

#[cfg(test)]
mod tests {
    use std::env;

    use netsim_packets::ieee80211::{parse_mac_address, FrameType, Ieee80211, Ieee80211ToAp};

    use super::*;

    /// Initializes a basic Hostapd instance for testing.
    fn init_hostapd() -> Hostapd {
        let (tx, _rx) = mpsc::channel(100);
        let config_path = env::temp_dir().join("hostapd.conf");
        Hostapd::new(tx, true, config_path)
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_generic() {
        // Sample 802.11 data frame for encryption/decryption test.
        let ieee80211 = Ieee80211ToAp {
            duration_id: 0,
            ftype: FrameType::Data,
            stype: 0,
            destination: parse_mac_address("2:2:2:2:2:2").unwrap(),
            source: parse_mac_address("1:1:1:1:1:1").unwrap(),
            bssid: parse_mac_address("0:0:0:0:0:0").unwrap(),
            seq_ctrl: 0,
            qos_ctrl: None,
            protected: 1,
            order: 0,
            more_frags: 0,
            retry: 0,
            pm: 0,
            more_data: 0,
            version: 0,
            payload: vec![0, 1, 2, 3, 4, 5], // Example payload
        }
        .try_into()
        .expect("Failed to create Ieee80211 frame");

        let hostapd = init_hostapd();

        // Encrypt and then decrypt the frame.
        let encrypted_frame = hostapd.try_encrypt(&ieee80211).expect("Encryption failed");
        let encrypted_view =
            Ieee80211::decode(&encrypted_frame).expect("Failed to decode encrypted frame");
        let decrypted_frame = hostapd.try_decrypt(&encrypted_view).expect("Decryption failed");

        // Create an expected frame with the protected bit cleared for comparison.
        let mut expected_frame_bytes = ieee80211.encode_to_vec().unwrap();
        // Clear the protected bit (byte 1, bit 6 i.e. 0x40)
        expected_frame_bytes[1] &= !0x40;

        // Verify that the decrypted frame is identical to the original frame (with
        // protected bit cleared).
        assert_eq!(
            decrypted_frame, expected_frame_bytes,
            "Decrypted frame does not match original frame (protected bit cleared)" /* More descriptive assertion message */
        );
    }

    #[tokio::test]
    async fn test_decrypt_encrypt_golden_frame() {
        // Read Golden Frame from shared test data (exported by packets crate via public
        // API) This relies on include_bytes! inside the packets crate, ensuring
        // consistent access.
        let pcap_bytes = netsim_packets::ieee80211::get_golden_ccmp_pcap();

        // Skip PCAP Header (24) + Packet Header (16) = 40 bytes
        // Note: Our generated pcap has exactly one packet.
        let encrypted_frame_bytes = &pcap_bytes[40..];
        const EXPECTED_DECRYPTED_FRAME_BYTES: [u8; 104] = [
            // Corrected array size to 104
            8, 1, 58, 1, 0, 19, 16, 133, 254, 1, 2, 21, 178, 0, 0, 0, 51, 51, 255, 197, 140, 97,
            192, 70, 170, 170, 3, 0, 0, 0, 134, 221, 96, 0, 0, 0, 0, 32, 58, 255, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 255, 197, 140, 97,
            135, 0, 44, 90, 0, 0, 0, 0, 254, 128, 0, 0, 0, 0, 0, 0, 121, 29, 23, 252, 71, 197, 140,
            97, 14, 1, 27, 50, 219, 39, 89, 3,
        ];

        let encrypted_ieee80211 = Ieee80211::decode(&encrypted_frame_bytes)
            .expect("Failed to decode encrypted Ieee80211 frame");

        let hostapd = init_hostapd();

        // Decrypt the golden encrypted frame.
        let decrypted_ieee80211 =
            hostapd.try_decrypt(&encrypted_ieee80211).expect("Decryption of golden frame failed");

        // Verify decryption against expected decrypted bytes.
        assert_eq!(
            decrypted_ieee80211,
            EXPECTED_DECRYPTED_FRAME_BYTES.to_vec(),
            "Decrypted golden frame does not match expected bytes"
        );

        // Re-encrypt the decrypted frame to verify round-trip.
        // We must re-enable the protected bit for try_encrypt to process it.
        let mut frame_to_encrypt = decrypted_ieee80211.clone();
        frame_to_encrypt[1] |= 0x40; // Set protected bit
        let decrypted_frame_view =
            Ieee80211::decode(&frame_to_encrypt).expect("Failed to decode decrypted frame");

        let reencrypted_frame = hostapd
            .try_encrypt(&decrypted_frame_view)
            .expect("Re-encryption of decrypted frame failed");

        assert_eq!(
            reencrypted_frame,
            encrypted_frame_bytes.to_vec(),
            "Re-encrypted frame does not match original encrypted frame"
        );

        // Re-decrypt again to ensure consistent round-trip decryption.
        let reencrypted_frame_view =
            Ieee80211::decode(&reencrypted_frame).expect("Failed to decode re-encrypted frame");

        let redecrypted_frame = hostapd
            .try_decrypt(&reencrypted_frame_view)
            .expect("Re-decryption of re-encrypted frame failed");

        assert_eq!(
            redecrypted_frame,
            EXPECTED_DECRYPTED_FRAME_BYTES.to_vec(),
            "Re-decrypted frame does not match expected bytes after re-encryption"
        );
    }
    // Implementation block for Hostapd specific to tests.
    impl Hostapd {
        /// Test-specific get_key: returns a fixed key for predictable
        /// encryption/decryption.
        pub fn get_key(&self, _ieee80211: &Ieee80211) -> (KeyData, usize, u8) {
            let mut key = [0u8; 32];
            const TEST_KEY: [u8; 16] = [
                // Defined test key as const for clarity
                202, 238, 127, 166, 61, 206, 22, 214, 17, 180, 130, 229, 4, 249, 255, 122,
            ];
            key[..16].copy_from_slice(&TEST_KEY);
            (key, 16, 0)
        }
    }
}
