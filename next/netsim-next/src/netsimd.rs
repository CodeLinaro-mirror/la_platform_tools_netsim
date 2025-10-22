// Copyright 2023-2025 The Android Open Source Project

use crate::config::IniFile;
use crate::logger;
use crate::platform;
use log::{error, info, warn};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tokio::task::JoinSet;

#[cfg(unix)]
use tokio::net::UnixListener;

const INI_FILENAME: &str = "netsim-next.ini";

// --- Platform-specific IPC Management ---

#[cfg(unix)]
/// Manages the lifecycle of the Unix domain socket.
/// Its `Drop` implementation ensures the socket file is cleaned up.
struct UnixSocketListener {
    path: PathBuf,
}

#[cfg(unix)]
impl UnixSocketListener {
    /// Creates a new listener, binds it, and spawns a task to accept connections.
    fn new(join_set: &mut JoinSet<()>) -> io::Result<Self> {
        let path = platform::get_runtime_dir().join("netsim.sock");
        if path.exists() {
            fs::remove_file(&path)?;
        }
        let listener = UnixListener::bind(&path)?;
        info!("Unix socket created at {}", path.display());

        join_set.spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((_stream, _addr)) => {
                        info!("Accepted new Unix socket connection");
                    }
                    Err(e) => {
                        error!("Failed to accept Unix socket connection: {}", e);
                        break;
                    }
                }
            }
        });
        Ok(Self { path })
    }
}

#[cfg(unix)]
impl Drop for UnixSocketListener {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_file(&self.path) {
            // If the file doesn't exist, it's not an error (e.g., clean shutdown).
            if e.kind() != io::ErrorKind::NotFound {
                warn!("Failed to remove Unix socket file '{}': {}", self.path.display(), e);
            }
        }
    }
}

// --- Main Daemon Logic ---

pub async fn run() -> bool {
    logger::init("netsim", true);
    info!("netsim startup");

    let mut ini_file = IniFile::new(&platform::get_runtime_dir(), INI_FILENAME);

    // 1. Lock: Attempt to acquire the lock.
    if let Err(e) = ini_file.try_lock() {
        error!(
            "Failed to acquire lock on {}. Another instance may be running. Error: {}",
            ini_file.path().display(),
            e
        );
        return false;
    }
    info!("Successfully acquired lock on {}", ini_file.path().display());

    // 2. Bind Sockets: Bind to port 0 to let the kernel assign available ports.
    let tcp_listener = match TcpListener::bind(("localhost", 0)).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Failed to bind TCP listener: {}", e);
            return false;
        }
    };
    let tcp_port = match tcp_listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(e) => {
            error!("Failed to get local address for TCP listener: {}", e);
            return false;
        }
    };

    // Placeholder for gRPC port assignment.
    let grpc_port = 0; // Replace with actual binding

    let mut config_data = HashMap::new();
    config_data.insert("tcp_port".to_string(), tcp_port.to_string());
    config_data.insert("grpc_port".to_string(), grpc_port.to_string());

    // VSOCK port is only applicable on Linux.
    #[cfg(target_os = "linux")]
    {
        let vsock_port = 0; // Replace with actual binding
        config_data.insert("vsock_port".to_string(), vsock_port.to_string());
        info!("Announced VSOCK port: {}", vsock_port);
    }

    // 3. Announce: Write the *actual* assigned ports for clients.
    if let Err(e) = ini_file.write(&config_data) {
        error!("Failed to write initial config to {}: {}", INI_FILENAME, e);
        return false;
    }

    info!("Announced TCP port: {}", tcp_port);
    info!("Announced gRPC port: {}", grpc_port);

    let mut join_set = JoinSet::new();

    // Spawn a task to handle TCP connections.
    join_set.spawn(async move {
        loop {
            if let Ok((_socket, _addr)) = tcp_listener.accept().await {
                info!("Accepted new TCP connection");
            }
        }
    });

    // Platform-specific IPC setup. The `_unix_listener` variable's scope
    // is the entire `run` function. When it's dropped on exit, the socket is cleaned up.
    #[cfg(unix)]
    let _unix_listener = match UnixSocketListener::new(&mut join_set) {
        Ok(listener) => Some(listener),
        Err(e) => {
            error!("Failed to create Unix socket listener: {}", e);
            return false;
        }
    };
    #[cfg(not(unix))]
    warn!("IPC is not supported on this platform.");

    let (server, client) = bluetooth::Server::new();
    join_set.spawn(server.run());
    info!("bluetooth server started");

    // Gracefully shut down the server
    if let Err(e) = client.shutdown().await {
        error!("Failed to send shutdown command: {}", e);
    }

    // Wait for all tasks in the JoinSet to complete
    while let Some(res) = join_set.join_next().await {
        if let Err(e) = res {
            error!("Server task panicked: {}", e);
        }
    }
    info!("netsim shutdown complete");

    true
}
