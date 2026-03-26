//! # Modem Simulator Server Logic
//!
//! This module implements the "server" personality of the `modem_simulator`
//! binary. See the project `README.md` for a full architectural overview.
//!
//! This module's responsibilities include:
//! - Daemonizing the server process.
//! - Managing the central `CellularNetworkSimulator` state.
//! - Listening for client connections on a TCP socket.
//! - Handling the handshake to register new modem connections.
//! - Processing AT commands from clients.
//! - Cleaning up all resources when a client disconnects.

use std::{collections::HashMap, sync::Arc};

use daemonize::Daemonize;
use modem_rs::{time::SystemClock, Callbacks, CellularNetworkSimulator, ModemId, NetworkCallbacks};
use serde::Serialize;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};
use tracing::{error, info};
use url::Url;

use crate::{ClientWriter, ServerCallbacks, PID_FILE, SERVER_LOG_FILE, TCP_PORT};

impl Callbacks for ServerCallbacks {
    fn send_at_response(&self, modem_id: ModemId, response: &[u8]) {
        let writers = self.client_writers.clone();
        let response = response.to_vec();
        tokio::spawn(async move {
            let writers = writers.lock().await;
            if let Some(writer) = writers.get(&modem_id) {
                let mut writer = writer.lock().await;
                if writer.write_all(&response).await.is_err() {
                    error!("[Server] Failed to send response to modem_id: {}", modem_id);
                }
            }
        });
    }
}

// A temporary callback handler for one-shot CI commands.
struct InjectorCallbacks {
    writer: ClientWriter,
}

impl Callbacks for InjectorCallbacks {
    fn send_at_response(&self, _modem_id: ModemId, response: &[u8]) {
        let writer = self.writer.clone();
        let response = response.to_vec();
        tokio::spawn(async move {
            let mut writer = writer.lock().await;
            if writer.write_all(&response).await.is_err() {
                error!("[Server] Failed to send response to injector client.");
            }
        });
    }
}

impl NetworkCallbacks for ServerCallbacks {
    fn on_new_remote_connection(&self, _modem_id: ModemId, _destination: String) {}
    fn on_modem_hanged_up(&self, _modem_id: ModemId) {}
}

#[derive(Serialize)]
struct ModemInfo {
    instance_id: u32,
    local_modem_index: u64,
    global_modem_id: u64,
    // metrics: MetricsSnapshot,
}

pub async fn run() {
    info!("[Server] Attempting to daemonize...");
    let stdout = std::fs::File::create(SERVER_LOG_FILE).unwrap();
    let stderr = std::fs::File::create(SERVER_LOG_FILE).unwrap();

    let daemonize = Daemonize::new()
        .pid_file(PID_FILE)
        .chown_pid_file(true)
        .working_directory("/tmp")
        .stdout(stdout)
        .stderr(stderr);

    match daemonize.start() {
        Ok(_) => info!("[Server] Daemonized successfully."),
        Err(e) => {
            error!("[Server] Daemonization failed: {}", e);
            return;
        }
    }

    info!("[Server] Starting modem simulator in server mode...");
    let listener = TcpListener::bind(format!("localhost:{}", TCP_PORT))
        .await
        .expect("Failed to bind to server port");
    info!("[Server] Listening on localhost:{}", TCP_PORT);

    let client_writers = Arc::new(Mutex::new(HashMap::new()));
    let callbacks = Arc::new(ServerCallbacks { client_writers: client_writers.clone() });
    let manager = CellularNetworkSimulator::new(callbacks);

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("[Server] Accepted new client connection from {:?}", addr);
                let manager = manager.clone();
                let client_writers = client_writers.clone();
                tokio::spawn(handle_client(stream, manager, client_writers));
            }
            Err(e) => {
                error!("[Server] Failed to accept client connection: {}", e);
            }
        }
    }
}

async fn handle_client(
    stream: TcpStream,
    manager: Arc<CellularNetworkSimulator>,
    client_writers: Arc<Mutex<HashMap<ModemId, ClientWriter>>>,
) {
    let (reader, writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    let writer_arc = Arc::new(Mutex::new(writer));

    let mut handshake_buf = Vec::new();
    loop {
        let bytes_read = reader.read_until(b'\n', &mut handshake_buf).await.unwrap_or(0);
        if bytes_read == 0 || handshake_buf.ends_with(b"\r\n\r\n") {
            break;
        }
    }

    let handshake = String::from_utf8_lossy(&handshake_buf);
    let (verb, path, modem_id_opt) = parse_handshake(&handshake);

    match (verb.as_str(), path.as_str(), modem_id_opt) {
        ("GET", "/modems", _) => {
            info!("[Server] Handling list-modems request.");
            let mut modem_infos = Vec::new();
            for modem_id in manager.get_modem_ids() {
                // TODO: Re-enable metrics once the feature is fully implemented in modem-rs.
                // let metrics = manager.get_modem(modem_id).unwrap().get_metrics();
                modem_infos.push(ModemInfo {
                    instance_id: (modem_id >> 32) as u32,
                    local_modem_index: (modem_id & 0xFFFFFFFF) as u64,
                    global_modem_id: modem_id as u64,
                    // metrics,
                });
            }
            let json_body = serde_json::to_string_pretty(&modem_infos).unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                json_body.len(),
                json_body
            );
            let mut writer = writer_arc.lock().await;
            writer.write_all(response.as_bytes()).await.unwrap_or_else(|e| {
                error!("[Server] Failed to send list-modems response: {}", e);
            });
        }
        ("REGISTER", "/modem", Some(modem_id)) => {
            info!("[Server] Registered proxy client for modem_id: {}", modem_id);
            if manager.get_modem(modem_id).is_none() {
                let modem_callbacks =
                    Arc::new(ServerCallbacks { client_writers: client_writers.clone() });
                manager.new_modem(modem_id, modem_callbacks).unwrap();
            }
            client_writers.lock().await.insert(modem_id, writer_arc);

            let mut reader = reader.into_inner();
            loop {
                let mut buf = [0; 1024];
                match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        manager.send_at_command(modem_id, &buf[..n]);
                    }
                    Err(_) => break,
                }
            }
            info!("[Server] Proxy client for modem_id {} disconnected. Cleaning up.", modem_id);
            client_writers.lock().await.remove(&modem_id);
            manager.remove_modem(modem_id);
        }
        ("INJECT", "/modem", Some(modem_id)) => {
            info!("[Server] Registered injector client for modem_id: {}", modem_id);
            let mut reader = reader.into_inner();
            loop {
                let mut buf = [0; 1024];
                match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        manager.send_at_command(modem_id, &buf[..n]);
                    }
                    Err(_) => break,
                }
            }
            info!("[Server] Injector client for modem_id {} disconnected.", modem_id);
        }
        _ => {
            error!("[Server] Client connection failed: invalid handshake.");
        }
    }
}

fn parse_handshake(handshake: &str) -> (String, String, Option<ModemId>) {
    let default = ("".to_string(), "".to_string(), None);
    let request_line = match handshake.lines().next() {
        Some(line) => line,
        None => return default,
    };
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return default;
    }
    let verb = parts[0].to_string();
    let path_and_query = parts[1];
    let url = match Url::parse(&format!("http://localhost{}", path_and_query)) {
        Ok(url) => url,
        Err(_) => return (verb, path_and_query.to_string(), None),
    };
    let path = url.path().to_string();
    for (key, value) in url.query_pairs() {
        if key == "modem_id" {
            return (verb, path, value.parse().ok());
        }
    }
    (verb, path, None)
}
