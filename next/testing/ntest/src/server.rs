use std::sync::Arc;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, UdpSocket},
};
use tokio_util::sync::CancellationToken;

use crate::common::{BUFFER_SIZE_TCP, BUFFER_SIZE_UDP};

/// Runs an echo server (TCP and UDP) on the specified port.
/// Returns the assigned port.
pub async fn run_server(port: u16, token: CancellationToken) -> anyhow::Result<u16> {
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
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = token.cancelled() => break,
                res = tcp_listener.accept() => {
                    if let Ok((mut socket, _)) = res {
                        let inner_token = token.clone();
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
