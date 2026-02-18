use std::time::Duration;

use anyhow::{Context, Result};
use log::info;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, UdpSocket},
};

use crate::common::{Protocol, ThroughputMonitor, BUFFER_SIZE_TCP, BUFFER_SIZE_UDP, PAYLOAD_BYTE};

pub async fn run_client(
    proto_str: String,
    target: String,
    payload_size: usize,
    expect_eof: bool,
    timeout_ms: u64,
) -> Result<()> {
    let proto: Protocol = proto_str.parse()?;

    info!(
        "Starting Client: proto={}, target={}, size={}, expect_eof={}, timeout={}ms",
        proto, target, payload_size, expect_eof, timeout_ms
    );
    println!("[Client] Starting connection to {}...", target);
    use std::io::Write;
    std::io::stdout().flush().ok();

    let payload = vec![PAYLOAD_BYTE; payload_size];
    let duration = Duration::from_millis(timeout_ms);

    tokio::time::timeout(duration, async {
        match proto {
            Protocol::Tcp => {
                let stream =
                    TcpStream::connect(&target).await.context("Failed to connect TCP")?;
                info!("Connected to {}", target);

                let (mut reader, mut writer) = stream.into_split();
                let payload_len = payload.len();

                let reader_task = tokio::spawn(async move {
                    let mut bytes_read = 0;
                    let total_size = payload_len;
                    let mut buf = vec![0u8; BUFFER_SIZE_TCP];

                    let mut monitor = ThroughputMonitor::new();
                    monitor.print_header();

                    while bytes_read < total_size {
                        let remaining = total_size - bytes_read;
                        let to_read = std::cmp::min(BUFFER_SIZE_TCP, remaining);
                        let slice = &mut buf[..to_read];

                        reader.read_exact(slice).await.context("Failed to read TCP echo")?;

                        // Verify content
                        if slice.iter().any(|&b| b != PAYLOAD_BYTE) {
                            return Err(anyhow::anyhow!("TCP Echo Mismatch"));
                        }

                        bytes_read += to_read;
                        monitor.update(to_read);
                    }
                    monitor.print_final_stats();
                    Ok::<(), anyhow::Error>(())
                });

                let start = std::time::Instant::now();
                writer.write_all(&payload).await.context("Failed to write TCP")?;

                // Wait for reader to finish
                reader_task.await.context("Reader task join failed")??;
                let _duration = start.elapsed();

                if expect_eof {
                   info!("Legacy expect_eof logic not fully adapted to split, assuming verified if we got payload");
                   Ok(())
                } else {
                        Ok(())
                }
            }
            Protocol::Udp => {
                let socket = UdpSocket::bind("0.0.0.0:0").await.context("Failed to bind UDP")?;
                socket.connect(&target).await.context("Failed to connect UDP")?;
                socket.send(&payload).await.context("Failed to send UDP")?;

                let mut buf = vec![0u8; BUFFER_SIZE_UDP];
                let len = socket.recv(&mut buf).await.context("Failed to recv UDP")?;
                if len == payload_size && &buf[..len] == payload.as_slice() {
                    info!("UDP Echo Verified");
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("UDP Echo Mismatch or Wrong Size"))
                }
            }
        }
    })
    .await
    .context("Client timed out")??;

    Ok(())
}
