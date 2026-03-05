// Copyright 2025 The Android Open Source Project

use actor_framework::ActorLifecycle;
use async_trait::async_trait;

use crate::link_actor::LinkActor;

#[async_trait]
impl ActorLifecycle for LinkActor {}
