use actor_framework::ActorLifecycle;
use async_trait::async_trait;

use crate::capture_actor::CaptureActor;

#[async_trait]
impl ActorLifecycle<netsim_model::chip::ChipId> for CaptureActor {
    type Error = crate::error::CaptureError;
}
