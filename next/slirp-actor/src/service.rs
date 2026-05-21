// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0
use std::collections::hash_map::Entry;

use actor_framework::{ActorService, DynContext};
use libslirp_rs::{LibSlirp, ProxyManager};

use crate::{
    error::SlirpError,
    slirp_actor::{ClientInfo, SlirpActor, SlirpCreate, SlirpReq, SlirpStatus},
};

impl ActorService for SlirpActor {
    type Id = u32;
    type Create = SlirpCreate;
    type Update = ();
    type Action = SlirpReq;
    type ActionResult = ();
    type Error = SlirpError;
    type Entity = SlirpStatus;
    type TypedStream = bytes::Bytes;

    async fn handle_create(
        &mut self,
        _id: Option<Self::Id>,
        _create: Self::Create,
        _: &mut DynContext<Self>,
    ) -> Result<Self::Id, Self::Error> {
        panic!("SlirpActor::create should not be called. Use RegisterSink action instead.")
    }

    async fn handle_get(
        &self,
        _id: Self::Id,
        _: &mut DynContext<Self>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        Ok(Some(SlirpStatus { initialized: self.libslirp.is_some() }))
    }

    async fn handle_update(
        &mut self,
        _id: Self::Id,
        _: Self::Update,
        _: &mut DynContext<Self>,
    ) -> Result<Self::Entity, Self::Error> {
        panic!("Update not supported")
    }

    async fn handle_delete(
        &mut self,
        _id: Self::Id,
        _: &mut DynContext<Self>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn handle_list(
        &mut self,
        _: &mut DynContext<Self>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(vec![])
    }

    async fn handle_action(
        &mut self,
        _id: Option<Self::Id>,
        action: Self::Action,
        ctx: &mut DynContext<Self>,
    ) -> Result<Self::ActionResult, Self::Error> {
        match action {
            SlirpReq::SendPacket(data) => {
                if let Some(slirp) = &self.libslirp {
                    slirp.input(data);
                } else {
                    panic!("SlirpActor: SendPacket called before Register");
                }
            }
            // Register a new Layer-2 client port (WiFi or Ethernet/Cellular).
            // - WiFiActor: Registers a single L2 port representing the wireless gateway interface.
            // - EthernetActor: Registers individual L2 ports representing each emulated guest
            //   Ethernet or Cellular chip.
            // Both actors exchange pre-formatted L2 802.3 Ethernet frames with the switch.
            SlirpReq::Register { client_id, stream, sink, notifier } => {
                if self.libslirp.is_none() {
                    let mut config = self.config.clone();
                    let mut proxy_manager = None;
                    let mut tx_proxy_bytes = None;

                    if let Some(ref proxy) = self.http_proxy {
                        let (tx, rx) = std::sync::mpsc::channel();
                        config.http_proxy_on = true;
                        let manager = http_proxy::Manager::new(proxy, rx)
                            .map_err(|e| crate::error::SlirpError::Internal(e.to_string()))?;
                        proxy_manager = Some(Box::new(manager) as Box<dyn ProxyManager>);
                        tx_proxy_bytes = Some(tx);
                    }

                    // Create internal downlink channel
                    let (downlink_tx, downlink_rx) = tokio::sync::mpsc::unbounded_channel();

                    // Wrap tx in Box<dyn PacketSender>, effectively removing the bridge thread
                    let slirp =
                        LibSlirp::new(config, Box::new(downlink_tx), proxy_manager, tx_proxy_bytes);
                    self.libslirp = Some(slirp);

                    // Register internal downlink stream as a typed stream
                    ctx.add_typed_stream(
                        0,
                        Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(downlink_rx)),
                    );
                }

                if let Entry::Vacant(e) = self.clients.entry(client_id) {
                    tracing::info!("SlirpActor: Registering client {client_id}");
                    e.insert(ClientInfo { sink, notifier });
                    ctx.add_stream(client_id, stream);
                } else {
                    tracing::warn!("SlirpActor: Register called twice for client {client_id}");
                }
            }
            SlirpReq::Unregister { client_id } => {
                tracing::info!("SlirpActor: Unregistering client {client_id}");
                self.clients.remove(&client_id);
                self.mac_table.retain(|_, v| *v != client_id);
                ctx.remove_stream(client_id);
            }
        }
        Ok(())
    }
}
