// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use netsim_proto::{frontend, frontend_grpc::FrontendServiceClient};
use protobuf::{well_known_types::empty, MessageField};
use verify_macros::{step, step_module};

use crate::orchestrator::TestContext;

#[step_module]
pub mod steps {
    use super::*;

    #[step(r#"@netsim links @(\S+) to @(\S+) by (\S+) RSSI (-?\d+) as "([^"]+)""#)]
    async fn netsim_create_link(
        w: &mut TestContext,
        sender: String,
        receiver: String,
        kind: String,
        rssi: i32,
        var_name: String,
    ) {
        let resolved_sender = w.resolve_placeholders(&format!("@{}", sender));
        let resolved_receiver = w.resolve_placeholders(&format!("@{}", receiver));
        let sender_device =
            w.netsim.map_actor_to_netsim(w, &resolved_sender).expect("got sender device name");
        let receiver_device =
            w.netsim.map_actor_to_netsim(w, &resolved_receiver).expect("got receiver device name");

        w.log_step(
            "@netsim",
            "->",
            &format!(
                "Links {} to {} by {} RSSI {} as \"{}\"",
                sender_device, receiver_device, kind, rssi, var_name
            ),
        );

        if w.is_dry_run {
            w.variables.insert(var_name, "dummy_link_id".to_string());
            return;
        }

        let client = w.get_or_create_grpc_client().expect("got grpc client");

        let (sender_id, receiver_id) =
            get_chip_ids(&client, &sender_device, &receiver_device, &kind).expect("found chip ids");

        let mut link = netsim_proto::model::Link::new();
        link.sender_id = sender_id;
        link.receiver_id = receiver_id;
        link.rssi = rssi;
        link.kind = kind_to_proto(&kind).into();

        let mut req = frontend::CreateLinkRequest::new();
        req.link = protobuf::MessageField::some(link);

        client.create_link(&req).expect("created link");

        let link_id = find_link_id(&client, sender_id, receiver_id, &kind).expect("found link id");
        w.variables.insert(var_name, link_id.to_string());
    }

    #[step(r#"@netsim should list a link with RSSI (-?\d+)"#)]
    async fn netsim_verify_link_rssi(w: &mut TestContext, expected_rssi: i32) {
        w.log_step("@netsim", "THEN", &format!("Should list a link with RSSI {}", expected_rssi));

        if w.is_dry_run {
            return;
        }

        let client = w.get_or_create_grpc_client().expect("got grpc client");
        let links_resp = client
            .list_link(&protobuf::well_known_types::empty::Empty::new())
            .expect("listed links");

        if !links_resp.links.iter().any(|link| link.rssi == expected_rssi) {
            panic!("Link with RSSI '{}' not found in ListLink response", expected_rssi);
        }
    }

    #[step(r#"@netsim patches link (\S+) with RSSI (-?\d+)"#)]
    async fn netsim_patch_link(w: &mut TestContext, link_var: String, rssi: i32) {
        let resolved_link_id = w.resolve_placeholders(&link_var);

        w.log_step(
            "@netsim",
            "->",
            &format!("Patches link {} with RSSI {}", resolved_link_id, rssi),
        );

        if w.is_dry_run {
            return;
        }

        let link_id = resolved_link_id.parse::<u32>().expect("valid link id");

        let client = w.get_or_create_grpc_client().expect("got grpc client");
        let links_resp = client
            .list_link(&protobuf::well_known_types::empty::Empty::new())
            .expect("listed links");
        let existing_link =
            links_resp.links.iter().find(|l| l.id == link_id).expect("found existing link");

        let mut link = netsim_proto::model::Link::new();
        link.sender_id = existing_link.sender_id;
        link.receiver_id = existing_link.receiver_id;
        link.rssi = rssi;
        link.kind = existing_link.kind;

        let mut req = frontend::PatchLinkRequest::new();
        req.link = protobuf::MessageField::some(link);
        req.id = link_id;

        client.patch_link(&req).expect("patched link");
    }

    #[step(r#"@netsim deletes link (\S+)"#)]
    async fn netsim_delete_link(w: &mut TestContext, link_var: String) {
        let resolved_link_id = w.resolve_placeholders(&link_var);

        w.log_step("@netsim", "->", &format!("Deletes link {}", resolved_link_id));

        if w.is_dry_run {
            return;
        }

        let link_id = resolved_link_id.parse::<u32>().expect("valid link id");
        let client = w.get_or_create_grpc_client().expect("got grpc client");

        let mut req = frontend::DeleteLinkRequest::new();
        req.id = link_id;

        client.delete_link(&req).expect("deleted link");
    }

    // Helper functions
    fn find_link_id(
        client: &netsim_proto::frontend_grpc::FrontendServiceClient,
        sender_id: u32,
        receiver_id: u32,
        kind: &str,
    ) -> Option<u32> {
        let links_resp = client.list_link(&protobuf::well_known_types::empty::Empty::new()).ok()?;
        links_resp
            .links
            .iter()
            .find(|link| {
                link.sender_id == sender_id
                    && link.receiver_id == receiver_id
                    && kind_matches(link.kind.enum_value_or_default(), kind)
            })
            .map(|link| link.id)
    }

    fn get_chip_ids(
        client: &netsim_proto::frontend_grpc::FrontendServiceClient,
        sender_device: &str,
        receiver_device: &str,
        kind: &str,
    ) -> Result<(u32, u32)> {
        let resp = client
            .list_device(&protobuf::well_known_types::empty::Empty::new())
            .map_err(|e| anyhow!("gRPC error: {:?}", e))?;
        let sender_id = find_chip_id(&resp, sender_device, kind)
            .ok_or_else(|| anyhow!("Sender chip not found"))?;
        let receiver_id = find_chip_id(&resp, receiver_device, kind)
            .ok_or_else(|| anyhow!("Receiver chip not found"))?;
        Ok((sender_id, receiver_id))
    }

    fn find_chip_id(
        resp: &frontend::ListDeviceResponse,
        device_name: &str,
        kind: &str,
    ) -> Option<u32> {
        resp.devices
            .iter()
            .find(|device| device.name == device_name)?
            .chips
            .iter()
            .find(|chip| kind_matches(chip.kind.enum_value_or_default(), kind))
            .map(|chip| chip.id)
    }

    fn kind_matches(kind: netsim_proto::common::ChipKind, kind_str: &str) -> bool {
        match kind_str {
            "bluetooth" => kind == netsim_proto::common::ChipKind::BLUETOOTH,
            "wifi" => kind == netsim_proto::common::ChipKind::WIFI,
            "uwb" => kind == netsim_proto::common::ChipKind::UWB,
            _ => false,
        }
    }

    fn kind_to_proto(kind_str: &str) -> netsim_proto::common::ChipKind {
        match kind_str {
            "bluetooth" => netsim_proto::common::ChipKind::BLUETOOTH,
            "wifi" => netsim_proto::common::ChipKind::WIFI,
            "uwb" => netsim_proto::common::ChipKind::UWB,
            _ => netsim_proto::common::ChipKind::UNSPECIFIED,
        }
    }
}

pub use steps::register_steps;
