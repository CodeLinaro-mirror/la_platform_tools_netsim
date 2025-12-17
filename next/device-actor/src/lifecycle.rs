use crate::device_actor::DeviceActor;
use actor_framework::ActorLifecycle;
use async_trait::async_trait;

#[async_trait]
impl ActorLifecycle<device_api::DeviceId> for DeviceActor {
    type Error = crate::error::DeviceError;
}
