// Copyright 2025 The Android Open Source Project

use crate::error::LinkError;
use crate::link_actor::LinkActor;
use actor_framework::ActorLifecycle;
use async_trait::async_trait;

#[async_trait]
impl ActorLifecycle for LinkActor {
    type Error = LinkError;
}
