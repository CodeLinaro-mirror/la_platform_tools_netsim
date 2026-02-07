use actor_framework::{ActorService, DynContext};
use async_trait::async_trait;

use crate::{
    error::SlirpError,
    slirp_actor::{SlirpActor, SlirpCreate, SlirpReq, SlirpStatus},
};

#[async_trait]
impl ActorService for SlirpActor {
    type Id = u32;
    type Create = SlirpCreate;
    type Update = ();
    type Action = SlirpReq;
    type ActionResult = ();
    type Error = SlirpError;
    type Entity = SlirpStatus;

    async fn handle_create(
        &mut self,
        _id: Option<Self::Id>,
        _create: Self::Create,
        _: &mut DynContext<Self::Id>,
    ) -> Result<Self::Id, Self::Error> {
        panic!("SlirpActor::create should not be called. Use RegisterSink action instead.")
    }

    async fn handle_get(
        &self,
        _id: Self::Id,
        _: &mut DynContext<Self::Id>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        Ok(Some(SlirpStatus { initialized: self.libslirp.is_some() }))
    }

    async fn handle_update(
        &mut self,
        _id: Self::Id,
        _: Self::Update,
        _: &mut DynContext<Self::Id>,
    ) -> Result<Self::Entity, Self::Error> {
        panic!("Update not supported")
    }

    async fn handle_delete(
        &mut self,
        _id: Self::Id,
        _: &mut DynContext<Self::Id>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn handle_list(
        &mut self,
        _: &mut DynContext<Self::Id>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(vec![])
    }

    async fn handle_action(
        &mut self,
        _id: Option<Self::Id>,
        action: Self::Action,
        ctx: &mut DynContext<Self::Id>,
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
                    let config = self.config.clone();
                    // Wrap tx in Box<dyn PacketSender>, effectively removing the bridge thread
                    let slirp =
                        libslirp_rs::libslirp::LibSlirp::new(config, Box::new(sink), None, None);
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
