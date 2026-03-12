use std::sync::Arc;

use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, UdpSocket},
};
use tokio_util::sync::CancellationToken;

use crate::{
    orchestrator::TestContext,
    types::{ClientParams, Throughput},
};

const BUFFER_SIZE_TCP: usize = 64 * 1024;
const BUFFER_SIZE_UDP: usize = 65535;

pub struct HostWorld {
    pub server_token: Option<CancellationToken>,
    pub last_port: Option<u16>,
    pub is_dry_run: bool,
}

impl HostWorld {
    pub async fn run_client(&mut self, _params: ClientParams) -> Result<Option<Throughput>> {
        Ok(None)
    }

    pub async fn reset_actor(&mut self) -> Result<()> {
        self.stop_server();
        Ok(())
    }

    pub fn get_label(&self) -> String {
        "host".to_string()
    }

    pub fn set_silent(&self, _silent: bool) {}
}

impl HostWorld {
    pub fn new(is_dry_run: bool) -> Self {
        Self { server_token: None, last_port: None, is_dry_run: is_dry_run }
    }

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
            let p = run_server(port, token.clone()).await?;
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
}

impl Drop for HostWorld {
    fn drop(&mut self) {
        self.stop_server();
    }
}

/// Runs an echo server (TCP and UDP) on the specified port.
/// Returns the assigned port.
async fn run_server(port: u16, token: CancellationToken) -> Result<u16> {
    let (tcp_listener, udp_socket, local_port) = loop {
        let listener = TcpListener::bind(("0.0.0.0", port)).await?;
        let p = listener.local_addr()?.port();
        match UdpSocket::bind(("0.0.0.0", p)).await {
            Ok(u) => break (listener, u, p),
            Err(e) => {
                if port != 0 {
                    return Err(e.into());
                }
                continue;
            }
        }
    };

    let udp_socket = Arc::new(udp_socket);

    // UDP Loop
    let udp_sock_clone = udp_socket.clone();
    let udp_token = token.clone();
    tokio::spawn(async move {
        let mut buf = [0u8; BUFFER_SIZE_UDP];
        loop {
            tokio::select! {
                _ = udp_token.cancelled() => break,
                res = udp_sock_clone.recv_from(&mut buf) => {
                    if let Ok((size, peer)) = res {
                        let _ = udp_sock_clone.send_to(&buf[..size], peer).await;
                    }
                }
            }
        }
    });

    // TCP Loop
    let tcp_token = token.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = tcp_token.cancelled() => break,
                res = tcp_listener.accept() => {
                    if let Ok((mut socket, _)) = res {
                        let inner_token = tcp_token.clone();
                        tokio::spawn(async move {
                            let mut buf = [0u8; BUFFER_SIZE_TCP];
                            loop {
                                tokio::select! {
                                    _ = inner_token.cancelled() => break,
                                    res = socket.read(&mut buf) => {
                                        match res {
                                            Ok(0) | Err(_) => break,
                                            Ok(n) => {
                                                if socket.write_all(&buf[..n]).await.is_err() { break; }
                                            }
                                        }
                                    }
                                }
                            }
                        });
                    }
                }
            }
        }
    });

    Ok(local_port)
}

/// STEP: When ^@host starts a (TCP|UDP) echo server on "(\w+)"$
async fn start_echo_server(w: &mut TestContext, proto: String, var_name: String) {
    let port = w.host.start_server(0).await.expect("Failed to start server");
    // Store full address (GatewayIP:Port) so usage {var} works directly
    let full_addr = format!("{}:{}", w.gateway_ip, port);
    w.log_step(
        "@host",
        "->",
        &format!("starts {} echo server on '{}'", proto.to_uppercase(), var_name),
    );
    w.set_variable(&var_name, full_addr);
}

/// STEP: Then ^@host receives (\d+)(KB|B|MB) (TCP|UDP) data(?: total)?$
async fn host_receives_data(w: &mut TestContext, size_val: usize, unit: String, proto: String) {
    let actor = "@host";
    w.log_step(actor, "THEN", &format!("Receives {}{} {} data", size_val, unit, proto));
}

/// STEP: Then ^@host receives all coordinated data$
async fn host_receives_coordinated_data(w: &mut TestContext) {
    let actor = "@host";
    w.log_step(actor, "THEN", "Receives all coordinated data");
}

// Include generated glue code
include!(env!("HOST_STEPS_GLUE"));
