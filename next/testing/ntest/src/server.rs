use std::sync::Arc;

use log::{error, info};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, UdpSocket},
};

use crate::common::{BUFFER_SIZE_TCP, BUFFER_SIZE_UDP};

pub async fn run_server(port: u16) -> anyhow::Result<()> {
    // Loop to find a port that works for both TCP and UDP
    let (tcp_listener, udp_socket, local_port) = loop {
        // Bind TCP first to get a port if 0 is specified
        let listener = TcpListener::bind(("0.0.0.0", port)).await?;
        let p = listener.local_addr()?.port();

        // Try to bind UDP to the same port
        match UdpSocket::bind(("0.0.0.0", p)).await {
            Ok(u) => break (listener, u, p),
            Err(e) => {
                if port != 0 {
                    // If a specific port was requested and failed, we can't retry on a different
                    // port.
                    return Err(e.into());
                }
                // If random port (0), retry.
                continue;
            }
        }
    };

    let udp_socket = Arc::new(udp_socket);

    info!("Starting ntest server on port {}", local_port);
    // Print to stdout for the test runner to pick up
    println!("SERVER_PORT={}", local_port);

    // Spawn UDP listener
    let udp_sock_clone = udp_socket.clone();
    tokio::spawn(async move {
        let mut buf = [0u8; BUFFER_SIZE_UDP];
        loop {
            match udp_sock_clone.recv_from(&mut buf).await {
                Ok((size, peer)) => {
                    info!("UDP received {} bytes from {}", size, peer);
                    if let Err(e) = udp_sock_clone.send_to(&buf[..size], peer).await {
                        error!("UDP send_to failed: {}", e);
                    }
                }
                Err(e) => {
                    error!("UDP recv_from failed: {}", e);
                }
            }
        }
    });

    // Handle TCP connections
    loop {
        match tcp_listener.accept().await {
            Ok((mut socket, peer)) => {
                info!("TCP connection from {}", peer);
                tokio::spawn(async move {
                    let mut buf = [0u8; BUFFER_SIZE_TCP];
                    loop {
                        match socket.read(&mut buf).await {
                            Ok(0) => return, // EOF
                            Ok(n) => {
                                if let Err(e) = socket.write_all(&buf[..n]).await {
                                    error!("TCP write failed: {}", e);
                                    return;
                                }
                            }
                            Err(e) => {
                                error!("TCP read failed: {}", e);
                                return;
                            }
                        }
                    }
                });
            }
            Err(e) => {
                error!("TCP accept failed: {}", e);
            }
        }
    }
}
