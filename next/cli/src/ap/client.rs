// Copyright 2026 The Android Open Source Project

use grpcio::CallOption;
use netsim_proto::{access_point, access_point_grpc::AccessPointServiceClient};
use protobuf::MessageField;

use super::{
    args::ApCommand,
    display::{print_ap, print_list_ap_response},
};
use crate::error::Result;

pub fn execute(cmd: &ApCommand, client: &AccessPointServiceClient, verbose: bool) -> Result<()> {
    match cmd {
        ApCommand::List(args) => {
            if args.continuous {
                loop {
                    let req = access_point::ListAccessPointsRequest::new();
                    let res = client.list_opt(&req, CallOption::default())?;
                    print_list_ap_response(&res, verbose);
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            } else {
                let req = access_point::ListAccessPointsRequest::new();
                let res = client.list_opt(&req, CallOption::default())?;
                print_list_ap_response(&res, verbose);
            }
        }
        ApCommand::Create(args) => {
            let mut ap = access_point::AccessPoint::new();
            if let Some(ssid) = &args.ssid {
                ap.ssid = ssid.clone();
            }
            if let Some(channel) = args.channel {
                ap.channel = channel;
            }
            if let Some(bssid) = &args.bssid {
                ap.bssid = bssid.clone();
            }
            if let Some(password) = &args.password {
                ap.wpa_passphrase = password.clone();
            }
            let mut req = access_point::CreateAccessPointRequest::new();
            req.access_point = MessageField::some(ap);

            let res = client.create_opt(&req, CallOption::default())?;
            if verbose {
                println!("Successfully created Access Point '{}' with ID {}", res.ssid, res.id);
                print_ap(&res);
            }
        }
        ApCommand::Patch(args) => {
            let mut req = access_point::UpdateAccessPointRequest::new();
            req.id = args.id;
            if let Some(ssid) = &args.ssid {
                req.ssid = Some(ssid.clone());
            }
            if let Some(channel) = args.channel {
                req.channel = Some(channel);
            }

            let res = client.update_opt(&req, CallOption::default())?;
            if verbose {
                println!("Successfully updated Access Point '{}' with ID {}", res.ssid, res.id);
                print_ap(&res);
            }
        }
        ApCommand::Remove(args) => {
            let mut req = access_point::DeleteAccessPointRequest::new();
            req.id = args.id;
            client.delete_opt(&req, CallOption::default())?;
            if verbose {
                println!("Successfully removed Access Point");
            }
        }
        ApCommand::Disconnect(args) => {
            let mut req = access_point::ExecuteAccessPointRequest::new();
            req.id = args.id;
            let mut disconnect = access_point::DisconnectRequest::new();
            disconnect.mac_address = args.mac_address.clone();
            req.action =
                Some(access_point::execute_access_point_request::Action::Disconnect(disconnect));
            client.execute_opt(&req, CallOption::default())?;
            if verbose {
                println!("Successfully disconnected client from Access Point");
            }
        }
    }
    Ok(())
}
