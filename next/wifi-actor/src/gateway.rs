// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use actor_framework::DynContext;
use ap_actor::SharedKeyStore;
use netsim_model::ChipId;
use netsim_packets::Ieee80211;

use crate::{error::WifiError, medium::Medium, wifi_actor::WifiActor};

pub const ETHERNET_HEADER_LEN: usize = 14;

/// Abstract interface for different network backends (e.g. TAP vs Slirp).
///
/// The `WifiActor` uses a single active gateway to route "Infrastructure"
/// traffic (traffic that goes to/from the external network, as opposed to
/// peer-to-peer).
///
/// - `TapGateway`: Bridges traffic to a host TAP interface (kernel).
/// - `SlirpGateway`: Bridges traffic to `SlirpActor` (user-mode networking).

#[async_trait::async_trait]
pub trait GatewayTrait: Send + Sync + std::fmt::Debug {
    async fn send_80211(&self, chip_id: ChipId, ieee80211: &Ieee80211) -> Result<usize, WifiError>;
    fn should_handle(&self, chip_id: ChipId) -> bool;
    fn handle_incoming(
        &self,
        chip_id: ChipId,
        packet: bytes::Bytes,
        medium: &mut Medium,
        shared_keys: &SharedKeyStore,
        out_queue: &mut Vec<(u32, bytes::Bytes)>,
    );
    async fn on_start(&mut self, ctx: &mut DynContext<WifiActor>);
    async fn on_chip_create(&mut self, chip_id: ChipId, ctx: &mut DynContext<WifiActor>);
    async fn on_chip_remove(&mut self, chip_id: ChipId, ctx: &mut DynContext<WifiActor>);
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
