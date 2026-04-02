// Copyright 2022 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Command Line Interface for Netsim

mod ap;
mod args;
mod ble;
mod browser;
mod display;
mod error;
mod file_handler;
mod grpc_client;
mod requests;
mod response;

use std::{env, fs::File, path::PathBuf};

use args::{GetCapture, NetsimArgs};
use clap::Parser;
use common::util::{ini_file::get_server_address, netsim_logger, os_utils::get_instance};
use file_handler::FileHandler;
use grpcio::{ChannelBuilder, EnvBuilder};
use netsim_proto::{
    access_point_grpc::AccessPointServiceClient, ble_service_grpc::BleServiceClient, frontend,
    frontend_grpc::FrontendServiceClient,
};
use tracing::error;

use crate::{
    error::{Error, Result},
    grpc_client::{ClientResponseReader, GrpcRequest, GrpcResponse},
};
// helper function to process streaming Grpc request
fn perform_streaming_request(
    client: &FrontendServiceClient,
    cmd: &mut GetCapture,
    req: &frontend::GetCaptureRequest,
    filename: &str,
) -> Result<()> {
    let dir = if cmd.location.is_some() {
        PathBuf::from(cmd.location.to_owned().unwrap())
    } else {
        env::current_dir().unwrap()
    };
    let output_file = dir.join(filename);
    cmd.current_file = output_file.display().to_string();
    grpc_client::get_capture(
        client,
        req,
        &mut ClientResponseReader {
            handler: Box::new(FileHandler {
                file: File::create(&output_file).map_err(|e| {
                    Error::from(format!("Failed to create file {}: {}", output_file.display(), e))
                })?,
                path: output_file,
            }),
        },
    )
}

/// helper function to send the Grpc request(s) and handle the response(s) per
/// the given command
fn perform_command(
    command: &mut args::Command,
    client: FrontendServiceClient,
    verbose: bool,
) -> Result<()> {
    // Get command's gRPC request(s)
    let requests = match command {
        args::Command::Capture(args::Capture::Patch(_) | args::Capture::Get(_))
        | args::Command::Link(_) => command.get_requests(&client),
        args::Command::Beacon(args::Beacon::Remove(_)) => {
            Ok(vec![args::Command::Devices(args::Devices { continuous: false }).get_request()])
        }
        _ => Ok(vec![command.get_request()]),
    }?;
    let mut process_error = false;
    // Process each request
    for (i, req) in requests.iter().enumerate() {
        let result = match command {
            // Continuous option sends the gRPC call every second
            args::Command::Devices(ref cmd) if cmd.continuous => {
                continuous_perform_command(command, &client, req, verbose)?;
                unreachable!("Continuous command should loop forever until error");
            }
            args::Command::Capture(args::Capture::List(ref cmd)) if cmd.continuous => {
                continuous_perform_command(command, &client, req, verbose)?;
                unreachable!("Continuous command should loop forever until error");
            }
            // Get Capture use streaming gRPC reader request
            args::Command::Capture(args::Capture::Get(ref mut cmd)) => {
                let GrpcRequest::GetCapture(request) = req else {
                    return Err(format!("Expected GetCaptureRequest. Got: {req:?}").into());
                };
                perform_streaming_request(&client, cmd, request, &cmd.filenames[i].to_owned())?;
                Ok(None)
            }
            args::Command::Beacon(args::Beacon::Remove(ref cmd)) => {
                let response = grpc_client::send_grpc(&client, &GrpcRequest::ListDevice)?;
                let GrpcResponse::ListDevice(response) = response else {
                    return Err(format!("Expected ListDeviceResponse. Got: {response:?}").into());
                };
                let id = find_id_for_remove(response, cmd)?;
                let res = grpc_client::send_grpc(
                    &client,
                    &GrpcRequest::DeleteDevice(frontend::DeleteDeviceRequest {
                        id,
                        ..Default::default()
                    }),
                )?;
                Ok(Some(res))
            }
            // All other commands use a single gRPC call
            _ => {
                let response = grpc_client::send_grpc(&client, req)?;
                Ok(Some(response))
            }
        };
        if let Err(e) = process_result(command, result, Some(req), verbose) {
            error!("{e}");
            process_error = true;
        };
    }
    if process_error {
        return Err("Not all requests were processed successfully.".into());
    }
    Ok(())
}

fn find_id_for_remove(
    response: frontend::ListDeviceResponse,
    cmd: &args::BeaconRemove,
) -> Result<u32> {
    let devices = response.devices;
    let device = devices
        .iter()
        .find(|device| device.name == cmd.device_name)
        .ok_or_else(|| format!("Device not found: {}", cmd.device_name))?;
    Ok(device.id)
}

/// Continuously execute the command every second
fn continuous_perform_command(
    command: &args::Command,
    client: &FrontendServiceClient,
    grpc_request: &GrpcRequest,
    verbose: bool,
) -> Result<()> {
    loop {
        let response = grpc_client::send_grpc(client, grpc_request)?;
        process_result(command, Ok(Some(response)), Some(grpc_request), verbose)?;
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
/// Check and handle the gRPC call result
fn process_result(
    command: &args::Command,
    result: Result<Option<GrpcResponse>>,
    request: Option<&GrpcRequest>,
    verbose: bool,
) -> Result<()> {
    match result {
        Ok(grpc_response) => {
            let response = grpc_response.unwrap_or(GrpcResponse::Unknown);
            command.print_response(&response, request, verbose)
        }
        Err(e) => Err(format!("Grpc call error: {e}").into()),
    }
}
/// Standard Netsim CLI entry point
fn main() {
    let mut args = NetsimArgs::parse();
    netsim_logger::init("netsim", args.verbose);
    if matches!(args.command, args::Command::Gui) {
        println!("Opening netsim web UI on default web browser");
        browser::open("http://localhost:7681/");
        return;
    } else if matches!(args.command, args::Command::Artifact) {
        let artifact_dir = common::system::netsimd_temp_dir();
        println!("netsim artifact directory: {}", artifact_dir.display());
        browser::open(artifact_dir);
        return;
    } else if matches!(args.command, args::Command::Bumble) {
        println!("Opening Bumble Hive on default web browser");
        browser::open("https://google.github.io/bumble/hive/index.html");
        return;
    }
    let server = match (args.vsock, args.port) {
        (Some(vsock), _) => format!("vsock:{vsock}"),
        (_, Some(port)) => format!("localhost:{port}"),
        _ => get_server_address(get_instance(args.instance)).unwrap_or_default(),
    };
    let channel =
        ChannelBuilder::new(std::sync::Arc::new(EnvBuilder::new().build())).connect(&server);
    let frontend_client = FrontendServiceClient::new(channel.clone());
    let access_point_client = AccessPointServiceClient::new(channel.clone());
    let ble_client = BleServiceClient::new(channel);

    if let args::Command::Ap(ap_cmd) = &args.command {
        if let Err(e) = crate::ap::client::execute(ap_cmd, &access_point_client, args.verbose) {
            error!("{e}");
        }
        return;
    }

    if let args::Command::Ble(ble_cmd) = &args.command {
        if let Err(e) = crate::ble::client::execute(ble_cmd, &ble_client, args.verbose) {
            error!("{e}");
        }
        return;
    }

    if let Err(e) = perform_command(&mut args.command, frontend_client, args.verbose) {
        error!("{e}");
    }
}

#[cfg(test)]
mod tests {
    use netsim_proto::{frontend::ListDeviceResponse, model::Device as DeviceProto};

    use crate::{args::BeaconRemove, find_id_for_remove};

    #[test]
    fn test_remove_device() {
        let device_name = String::from("a-device");
        let device_id = 42;

        let cmd = &BeaconRemove { device_name: device_name.clone() };

        let response = ListDeviceResponse {
            devices: vec![DeviceProto { id: device_id, name: device_name, ..Default::default() }],
            ..Default::default()
        };

        let id = find_id_for_remove(response, cmd);
        assert!(id.is_ok(), "{}", id.unwrap_err());
        let id = id.unwrap();

        assert_eq!(device_id, id);
    }
}
