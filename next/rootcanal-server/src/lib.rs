// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Legacy Rootcanal control port server.
//!
//! This TCP server is required in virtualized Cuttlefish environments to handle
//! commands from host-side test runners (such as pts-bot/mmi2grpc) which
//! dynamically configure devices and simulate physical medium range changes.

use std::{fmt::Write as _, io, net::SocketAddr};

use device_actor::DeviceClient;
use netsim_model::{ChipKind, ChipUpdate, DeviceId, DeviceUpdate};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tracing::{error, info, warn};

/// Binds the TCP listener to the requested port.
pub fn bind(port: u16) -> io::Result<TcpListener> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let std_listener = std::net::TcpListener::bind(addr)?;
    std_listener.set_nonblocking(true)?;
    TcpListener::from_std(std_listener)
}

/// Runs the TCP server loop, accepting connections and spawning handlers.
pub async fn run(listener: TcpListener, device_client: DeviceClient) {
    info!("Rootcanal legacy control server started on {}", listener.local_addr().unwrap());
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("Accepted Rootcanal control connection from {}", addr);
                let client = device_client.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, client).await {
                        error!("Error handling Rootcanal client {}: {}", addr, e);
                    }
                    info!("Rootcanal control connection closed for {}", addr);
                });
            }
            Err(e) => {
                error!("Error accepting Rootcanal control connection: {}", e);
                // Mitigate tight CPU spin in case of persistent accept errors (e.g. EMFILE)
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }
}

/// Handles an individual Rootcanal control connection.
async fn handle_client(mut stream: TcpStream, device_client: DeviceClient) -> io::Result<()> {
    let welcome = b"RootCanal\n";
    let size = welcome.len() as u32;
    stream.write_all(&size.to_le_bytes()).await?;
    stream.write_all(welcome).await?;
    stream.flush().await?;

    loop {
        let mut name_len_buf = [0u8; 1];
        match stream.read_exact(&mut name_len_buf).await {
            Ok(_) => {}
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                // Client disconnected gracefully
                break;
            }
            Err(e) => return Err(e),
        }
        let name_len = name_len_buf[0] as usize;

        let mut name_buf = [0u8; 256];
        stream.read_exact(&mut name_buf[..name_len]).await?;
        let name = std::str::from_utf8(&name_buf[..name_len])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut args_len_buf = [0u8; 1];
        stream.read_exact(&mut args_len_buf).await?;
        let args_len = args_len_buf[0] as usize;

        // Read arguments into stack buffers to avoid heap allocations and borrow
        // checker issues
        let mut parsed_args = [[0u8; 128]; 4];
        let mut parsed_args_len = [0usize; 4];
        let mut parsed_args_present = [false; 4];

        for i in 0..args_len {
            let mut arg_len_buf = [0u8; 1];
            stream.read_exact(&mut arg_len_buf).await?;
            let arg_len = arg_len_buf[0] as usize;

            if i < parsed_args.len() {
                if arg_len > parsed_args[i].len() {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "Argument too long"));
                }
                stream.read_exact(&mut parsed_args[i][..arg_len]).await?;
                parsed_args_len[i] = arg_len;
                parsed_args_present[i] = true;
            } else {
                // Skip/discard argument if we have too many
                let mut discard = vec![0u8; arg_len];
                stream.read_exact(&mut discard).await?;
            }
        }

        info!("Received Rootcanal control command: {} with {} args", name, args_len);

        // Handle CLOSE_TEST_CHANNEL immediately (no response)
        if name == "CLOSE_TEST_CHANNEL" {
            break;
        }

        let device_id_raw = if parsed_args_present[0] {
            let len = parsed_args_len[0];
            std::str::from_utf8(&parsed_args[0][..len]).ok().and_then(|s| s.parse::<u32>().ok())
        } else {
            None
        };

        let phy_index_raw = if parsed_args_present[1] {
            let len = parsed_args_len[1];
            std::str::from_utf8(&parsed_args[1][..len]).ok()
        } else {
            None
        };

        let response_str = match name {
            "list" => handle_list(&device_client).await,
            "del_device_from_phy" => {
                handle_phy_state(&device_client, device_id_raw, phy_index_raw, false).await
            }
            "add_device_to_phy" => {
                handle_phy_state(&device_client, device_id_raw, phy_index_raw, true).await
            }
            "set_device_configuration" => "OK".to_string(),
            other => {
                warn!("Unknown legacy Rootcanal command received: {}", other);
                "OK".to_string()
            }
        };

        // Send response packet ([4-byte size][payload])
        let response_bytes = response_str.as_bytes();
        let resp_size = response_bytes.len() as u32;
        stream.write_all(&resp_size.to_le_bytes()).await?;
        stream.write_all(response_bytes).await?;
        stream.flush().await?;
    }

    Ok(())
}

/// Handles the legacy 'list' command.
/// Format response to match Rootcanal device & physical simulation layers.
async fn handle_list(device_client: &DeviceClient) -> String {
    let mut devices_section = String::new();
    let mut low_energy_section = String::new();
    let mut br_edr_section = String::new();

    if let Ok(list_resp) = device_client.list().await {
        for device in list_resp.devices.iter() {
            let _ = writeln!(devices_section, "  {}:hci_device_{}", device.id, device.id);

            let has_enabled_bluetooth =
                device.chips.iter().any(|chip| chip.kind == ChipKind::BLUETOOTH && chip.enabled);

            if has_enabled_bluetooth {
                if !low_energy_section.is_empty() {
                    let _ = write!(low_energy_section, ",");
                }
                let _ = write!(low_energy_section, "{}", device.id);

                if !br_edr_section.is_empty() {
                    let _ = write!(br_edr_section, ",");
                }
                let _ = write!(br_edr_section, "{}", device.id);
            }
        }
    }

    format!(
        "Devices:\n{}Phys:\n  0:LOW_ENERGY:{}\n  1:BR_EDR:{}\n",
        devices_section, low_energy_section, br_edr_section
    )
}

/// Handles 'del_device_from_phy' (enabled = false) and 'add_device_to_phy'
/// (enabled = true).
///
/// Note: The `phy_index_raw` argument specifies the physical medium (0=LE,
/// 1=Classic). Currently, we toggle the entire Bluetooth chip's enabled status,
/// but we parse it for correctness/completeness.
async fn handle_phy_state(
    device_client: &DeviceClient,
    device_id_raw: Option<u32>,
    phy_index_raw: Option<&str>,
    enabled: bool,
) -> String {
    let device_id_raw = match device_id_raw {
        Some(id) => id,
        None => {
            warn!("Invalid or missing device_id in phy command");
            return "OK".to_string();
        }
    };

    if let Some(phy_str) = phy_index_raw {
        info!("Phy command for device {} targeted medium: {}", device_id_raw, phy_str);
    }

    let device_id = DeviceId(device_id_raw);
    let device = match device_client.get(device_id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            warn!("Device {} not found during phy state update", device_id_raw);
            return "OK".to_string();
        }
        Err(e) => {
            error!("Failed to get device {} from DeviceClient: {}", device_id_raw, e);
            return "OK".to_string();
        }
    };

    let chip_updates: Vec<ChipUpdate> = device
        .chips
        .iter()
        .filter(|chip| chip.kind == ChipKind::BLUETOOTH)
        .map(|chip| ChipUpdate {
            id: Some(chip.id.into()),
            enabled: Some(enabled),
            ..Default::default()
        })
        .collect();

    if chip_updates.is_empty() {
        info!("No Bluetooth chips found on device {} to toggle state", device_id_raw);
        return "OK".to_string();
    }

    let device_update =
        DeviceUpdate { id: device_id_raw, chips: Some(chip_updates), ..Default::default() };

    if let Err(e) = device_client.update(device_id, device_update).await {
        error!("Failed to update device {} chip states: {}", device_id_raw, e);
    } else {
        info!(
            "Successfully toggled Bluetooth chips to enabled={} on device {}",
            enabled, device_id_raw
        );
    }

    "OK".to_string()
}
