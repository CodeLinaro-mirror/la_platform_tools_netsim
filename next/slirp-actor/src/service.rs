use actor_framework::{ActorService, DynContext};
use libslirp_rs::libslirp::{LibSlirp, ProxyManager};

use crate::{
    error::SlirpError,
    slirp_actor::{SlirpActor, SlirpCreate, SlirpReq, SlirpStatus},
};

impl ActorService for SlirpActor {
    type Id = u32;
    type Create = SlirpCreate;
    type Update = ();
    type Action = SlirpReq;
    type ActionResult = ();
    type Error = SlirpError;
    type Entity = SlirpStatus;
    type TypedStream = ();

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
                    panic!("SlirpActor: SendPacket called before RegisterSink");
                }
            }
            SlirpReq::Register { stream, sink } => {
                if self.libslirp.is_some() {
                    panic!("SlirpActor: Register called twice");
                } else {
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
                    // Wrap tx in Box<dyn PacketSender>, effectively removing the bridge thread
                    let slirp =
                        LibSlirp::new(config, Box::new(sink), proxy_manager, tx_proxy_bytes);
                    self.libslirp = Some(slirp);

                    // Add the stream to the context
                    // We use 0 as the ID, or should we define a constant?
                    // lifecycle.rs uses 0 for now as it ignores id.
                    let stream_id = 0;
                    ctx.add_stream(stream_id, stream);
                }
            }
        }
        Ok(())
    }
}
