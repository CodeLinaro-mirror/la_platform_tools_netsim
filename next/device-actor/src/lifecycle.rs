use crate::device_actor::DeviceActor;
use actor_framework::ActorLifecycle;
use async_trait::async_trait;

#[async_trait]
impl ActorLifecycle for DeviceActor {
    type Error = crate::error::DeviceError;
}
