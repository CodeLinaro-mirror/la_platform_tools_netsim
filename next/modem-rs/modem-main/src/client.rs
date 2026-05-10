//! # Modem Simulator Client Logic
//!
//! This module implements the "client" personality of the `modem_simulator`
//! binary. See the project `README.md` for a full architectural overview.
//!
//! This module assumes that the central server is already running. Its sole job
//! is to act as an I/O proxy for one or more logical modems by performing the
//! following steps:
//! 1. Read the `VSOC_INSTANCE_ID` environment variable.
//! 2. For each file descriptor, compute a globally unique `ModemId`.
//! 3. Open a dedicated TCP connection to the server for each file descriptor.
//! 4. Perform the handshake to register the `ModemId`.
//! 5. Proxy data between the file descriptor and the TCP socket using
//!    `copy_bidirectional`.

use std::os::unix::io::FromRawFd;

use tokio::{
    io::{copy_bidirectional, AsyncWriteExt},
    net::TcpStream,
};
use tracing::{error, info};

use crate::{Args, TCP_PORT};

pub async fn run(args: Args) {
    let instance_num: u32 = args.instance_id.expect("Missing --instance_id in proxy mode");

    let mut join_handles = Vec::new();
    let server_fds = args.server_fds.expect("Missing --server-fds in proxy mode");
    for (i, fd_str) in server_fds.split(',').enumerate() {
        let local_modem_index = i as u64;
        let global_modem_id = ((instance_num as u64) << 32) | local_modem_index;
        let fd: i32 = fd_str.parse().expect("Invalid file descriptor");

        let handle = tokio::spawn(async move {
            let mut stream = TcpStream::connect(format!("localhost:{}", TCP_PORT))
                .await
                .expect("Client failed to connect to server");
            info!("[Client] Connected to server for modem_id {}", global_modem_id);

            let handshake = format!("CONNECT?modem_id={}\r\n\r\n", global_modem_id);
            stream.write_all(handshake.as_bytes()).await.unwrap();

            // SAFETY: This is safe because we assume the file descriptors passed by
            // the Cuttlefish environment are valid, open, and owned by this process.
            // The `File` object will take ownership and properly close the FD when
            // it goes out of scope.
            //            let std_file = unsafe { std::fs::File::from_raw_fd(fd) };
            //            let mut fd_stream = tokio::fs::File::from_std(std_file);

            info!("[Client] Starting proxy for global_modem_id: {}", global_modem_id);
            if let Err(e) = copy_bidirectional(&mut stream, &mut fd_stream).await {
                error!("[Client] Proxy for modem_id {} failed: {}", global_modem_id, e);
            }
            info!("[Client] Proxy for global_modem_id: {} finished.", global_modem_id);
        });
        join_handles.push(handle);
    }

    // Wait for all proxy tasks to complete. In a real scenario, this will run until
    // the Cuttlefish instance is shut down, which closes the file descriptors.
    for handle in join_handles {
        let _ = handle.await;
    }
}
