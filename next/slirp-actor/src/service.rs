// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0
use std::collections::hash_map::Entry;

use actor_framework::{ActorService, DynContext};
#[cfg(not(feature = "cuttlefish"))]
use libslirp_rs::{LibSlirp, ProxyManager};

use crate::{
    error::SlirpError,
    slirp_actor::{
        ClientInfo, SlirpActor, SlirpBackend, SlirpBackendInstance, SlirpCreate, SlirpReq,
        SlirpStatus,
    },
};

#[cfg(not(feature = "cuttlefish"))]
fn to_native_config(config: &libslirp_rs::SlirpConfig) -> slirp::Config {
    let mut guest_ipv6_octets = config.vprefix_addr6.octets();
    guest_ipv6_octets[15] = 1;
    let guest_ipv6 = std::net::Ipv6Addr::from(guest_ipv6_octets);

    let dns_search =
        if config.vdnssearch.is_empty() { None } else { Some(config.vdnssearch.clone()) };

    let dns_servers = config.host_dns.iter().map(|addr| addr.ip()).collect::<Vec<_>>();

    slirp::Config {
        host_ipv4: config.vhost,
        guest_ipv4: config.vdhcp_start,
        host_ipv6: config.vhost6,
        guest_ipv6,
        boot_file: config.bootfile.clone(),
        tftp_server_name: config.tftp_server_name.clone(),
        domain_name: config.vdomainname.clone(),
        dns_search,
        client_hostname: config.vhostname.clone(),
        dns_servers,
        tftp_root: config.tftp_path.clone(),
        ..Default::default()
    }
}

impl SlirpActor {
    fn init_backend(&mut self, ctx: &mut DynContext<Self>) -> Result<(), SlirpError> {
        match self.backend_type {
            #[cfg(not(feature = "cuttlefish"))]
            SlirpBackend::CFfi => self.init_c_ffi_backend(ctx),
            #[cfg(feature = "cuttlefish")]
            SlirpBackend::CFfi => {
                tracing::warn!(
                    "C-FFI backend requested on cuttlefish build; defaulting to Native slirp"
                );
                self.backend_type = SlirpBackend::Native;
                self.init_native_backend(ctx)
            }
            SlirpBackend::Native => self.init_native_backend(ctx),
        }
    }

    #[cfg(not(feature = "cuttlefish"))]
    fn init_c_ffi_backend(&mut self, ctx: &mut DynContext<Self>) -> Result<(), SlirpError> {
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
        let slirp = LibSlirp::new(config, Box::new(downlink_tx), proxy_manager, tx_proxy_bytes);
        self.backend_instance = Some(SlirpBackendInstance::CFfi(slirp));

        let stream_id = self.next_stream_id;
        self.next_stream_id += 1;
        self.active_stream_id = Some(stream_id);

        // Register internal downlink stream as a typed stream
        ctx.add_typed_stream(
            stream_id,
            Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(downlink_rx)),
        );
        Ok(())
    }

    fn init_native_backend(&mut self, ctx: &mut DynContext<Self>) -> Result<(), SlirpError> {
        #[cfg(feature = "cuttlefish")]
        let native_config = self.config.clone();
        #[cfg(not(feature = "cuttlefish"))]
        let native_config = to_native_config(&self.config);

        if self.http_proxy.is_some() {
            tracing::warn!("Native Slirp backend does not support HTTP proxy yet");
        }

        let (downlink_tx, downlink_rx) = tokio::sync::mpsc::unbounded_channel::<bytes::Bytes>();
        let (uplink_tx, uplink_rx) = tokio::sync::mpsc::unbounded_channel::<bytes::Bytes>();

        self.backend_instance = Some(SlirpBackendInstance::Native { uplink_tx });

        let stream_id = self.next_stream_id;
        self.next_stream_id += 1;
        self.active_stream_id = Some(stream_id);

        ctx.add_typed_stream(
            stream_id,
            Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(downlink_rx)),
        );

        let task_id = self.next_task_id;
        self.next_task_id += 1;
        self.active_task_id = Some(task_id);

        ctx.spawn(
            task_id,
            Box::pin(async move {
                crate::native::run_native_slirp_loop(native_config, downlink_tx, uplink_rx).await;
                task_id
            }),
        );

        Ok(())
    }
}

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
        Ok(Some(SlirpStatus { initialized: self.backend_instance.is_some() }))
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
                if let Some(instance) = &self.backend_instance {
                    instance.input(data);
                } else {
                    panic!("SlirpActor: SendPacket called before Register");
                }
            }
            // Register a new Layer-2 client port (WiFi or Ethernet/Cellular).
            // - WiFiActor: Registers a single L2 port representing the wireless gateway interface.
            // - EthernetActor: Registers individual L2 ports representing each emulated guest
            //   Ethernet or Cellular chip.
            // Both actors exchange pre-formatted L2 802.3 Ethernet frames with the switch.
            SlirpReq::Register { client_id, stream, mut sink, notifier } => {
                if self.backend_instance.is_none() {
                    self.init_backend(ctx)?;
                }

                if let Entry::Vacant(e) = self.clients.entry(client_id) {
                    tracing::info!("SlirpActor: Registering client {client_id}");
                    let (downlink_tx, mut downlink_rx) = tokio::sync::mpsc::unbounded_channel();
                    e.insert(ClientInfo { sink: downlink_tx, notifier });
                    ctx.add_stream(client_id, stream);

                    // Spawn isolated background forwarding task on SlirpActor's runtime context
                    ctx.spawn(
                        client_id,
                        Box::pin(async move {
                            use futures::sink::SinkExt;
                            while let Some(bytes) = downlink_rx.recv().await {
                                if sink.send(bytes).await.is_err() {
                                    break;
                                }
                            }
                            client_id
                        }),
                    );
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
            SlirpReq::SwitchBackend(new_backend) => {
                if self.backend_type != new_backend {
                    tracing::info!("SlirpActor: Switching backend to {:?}", new_backend);
                    if let Some(old_instance) = self.backend_instance.take() {
                        old_instance.shutdown();
                    }
                    self.backend_type = new_backend;
                    if !self.clients.is_empty() {
                        self.init_backend(ctx)?;
                    }
                }
            }
        }
        Ok(())
    }
}
