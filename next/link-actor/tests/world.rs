// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use actor_framework::FrameworkError;
use link_actor::{LinkActor, LinkClient};
use link_api::{Link, LinkCreate, LinkId, LinkUpdate};
use netsim_model::{ChipClient, ChipId, ChipKind, MockChipClient};

/// The BDD World for Link Actor tests.
///
/// Encapsulates the actor runtime, the client interaction, and the mock chip
/// clients. Dropping this struct terminates the actor task.
pub struct World {
    pub client: LinkClient,
    _actor_task: tokio::task::JoinHandle<()>,
}

impl Drop for World {
    fn drop(&mut self) {
        self._actor_task.abort();
    }
}

impl World {
    /// Creates a new World with default mock clients.
    pub async fn new() -> Self {
        let mut clients = HashMap::new();
        clients.insert(
            ChipKind::BLUETOOTH,
            Box::new(Self::create_default_mock()) as Box<dyn ChipClient>,
        );
        clients
            .insert(ChipKind::WIFI, Box::new(Self::create_default_mock()) as Box<dyn ChipClient>);
        clients.insert(ChipKind::UWB, Box::new(Self::create_default_mock()) as Box<dyn ChipClient>);
        Self::with_clients(clients).await
    }

    /// Creates a new World with injected custom mock clients.
    pub async fn with_clients(clients: HashMap<ChipKind, Box<dyn ChipClient>>) -> Self {
        let (actor, client) = link_actor::new();
        let link_actor_state = LinkActor::new(clients);
        let actor_task = tokio::spawn(actor.run(link_actor_state));
        World { _actor_task: actor_task, client }
    }

    // TODO: move this to a common place for Link and Device actors
    fn create_default_mock() -> MockChipClient {
        let mut mock = MockChipClient::new();
        mock.expect_read().returning(|_| Ok(netsim_model::Chip::default()));
        mock.expect_update().returning(|_, _| Ok(netsim_model::Chip::default()));
        mock.expect_create().returning(|_, _| Ok(()));
        mock.expect_delete().returning(|_| Ok(()));
        mock.expect_read_statistics().returning(|| Ok(Box::from([])));
        mock.expect_reset().returning(|_| Ok(netsim_model::Chip::default()));
        mock.expect_clone_box().returning(|| Box::new(Self::create_default_mock()));
        mock
    }

    /// BDD Step: Given default chips are added to the actor.
    pub async fn given_default_chips(&self) {
        self.client.notify_chip_added(ChipId(1), ChipKind::BLUETOOTH).await.unwrap();
        self.client.notify_chip_added(ChipId(2), ChipKind::BLUETOOTH).await.unwrap();
        self.client.notify_chip_added(ChipId(3), ChipKind::WIFI).await.unwrap();
    }

    /// BDD Step: When a link is created.
    pub async fn when_create_link(
        &self,
        sender: ChipId,
        receiver: ChipId,
        rssi: i8,
    ) -> Result<LinkId, FrameworkError<link_api::LinkError>> {
        let params = LinkCreate { sender, receiver, rssi };
        self.client.create(params).await
    }

    /// BDD Step: When a chip is added (notification).
    pub async fn when_notify_chip_added(&self, chip_id: ChipId, kind: ChipKind) {
        self.client.notify_chip_added(chip_id, kind).await.unwrap();
    }

    /// BDD Step: When a chip is removed (notification).
    pub async fn when_notify_chip_removed(&self, chip_id: ChipId) {
        self.client.notify_chip_removed(chip_id).await.unwrap();
    }

    /// BDD Step: When a link is updated.
    pub async fn when_update_link(
        &self,
        id: LinkId,
        rssi: i8,
    ) -> Result<Link, FrameworkError<link_api::LinkError>> {
        self.client.update(id, LinkUpdate { rssi: Some(rssi) }).await?;
        self.client.get(id).await.map(|opt| opt.expect("link exists"))
    }
}
