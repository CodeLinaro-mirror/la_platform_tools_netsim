// Copyright 2026 The Android Open Source Project

use futures_util::StreamExt;
use grpcio::CallOption;
use netsim_proto::{ble_service::ScanRequest, ble_service_grpc::BleServiceClient, model::Position};

use super::{args::BleCommand, display::print_scan_response};
use crate::error::Result;

pub fn execute(cmd: &BleCommand, client: &BleServiceClient, verbose: bool) -> Result<()> {
    match cmd {
        BleCommand::Scan(args) => {
            let mut req = ScanRequest::new();
            let mut pos = Position::new();
            pos.x = args.x;
            pos.y = args.y;
            pos.z = args.z;
            req.position = protobuf::MessageField::some(pos);

            let mut stream = client.scan_opt(&req, CallOption::default())?;

            println!("Starting BLE scan at ({}, {}, {})...", args.x, args.y, args.z);
            println!("Press Ctrl-C to stop.");

            futures::executor::block_on(async {
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(res) => print_scan_response(&res, verbose),
                        Err(e) => {
                            eprintln!("Scan stream disconnected or encountered an error: {}", e);
                            break;
                        }
                    }
                }
            });
        }
    }
    Ok(())
}
