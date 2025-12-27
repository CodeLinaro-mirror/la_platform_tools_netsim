use crate::capture_actor::CaptureActor;
use actor_framework::ActorLifecycle;
use async_trait::async_trait;

#[async_trait]
impl ActorLifecycle<netsim_model::chip::ChipId> for CaptureActor {
    type Error = crate::error::CaptureError;
}
