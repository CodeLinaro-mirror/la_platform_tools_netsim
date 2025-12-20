// Copyright 2025 The Android Open Source Project

#[cfg(test)]
mod tests {
    use crate::LinkActor;
    use actor_framework::{ActorService, BoxStream, Context};

    use link_api::{LinkCreate, LinkId};
    use netsim_model::chip::{ChipId, ChipKind};
    use std::time::Duration;

    struct MockContext;

    impl<Id> Context<Id> for MockContext
    where
        Id: Into<u32> + Send + 'static,
    {
        fn set_interval(&mut self, _duration: Duration) {}
        fn add_stream(&mut self, _id: Id, _stream: BoxStream) {}
        fn remove_stream(&mut self, _id: Id) {}
        fn shutdown(&mut self) {}
        fn abort(&mut self, _id: Id) {}
        fn spawn(
            &mut self,
            _id: Id,
            _task: std::pin::Pin<Box<dyn std::future::Future<Output = Id> + Send>>,
        ) {
            tokio::spawn(async move {
                let _ = _task.await;
            });
        }
    }

    #[tokio::test]
    async fn test_link_entity_lifecycle() {
        let mut actor = LinkActor::new();
        let sender = ChipId(1);
        let receiver = ChipId(2);

        // Pre-populate chip_kind_map to test kind inference
        actor.chip_kind_map.insert(sender, ChipKind::BLUETOOTH);
        actor.chip_kind_map.insert(receiver, ChipKind::BLUETOOTH);

        let link_id = LinkId(1);
        let params = LinkCreate { sender, receiver, rssi: -50 };

        let mut runtime = MockContext;

        // Create
        actor.handle_create(Some(link_id), params.clone(), &mut runtime).await.unwrap();

        // Verify lookup
        assert_eq!(actor.chip_pairs.get(&(params.sender, params.receiver)), Some(&link_id));

        // Verify kind set from params (in store)
        let link = actor.handle_get(link_id, &mut runtime).await.unwrap().unwrap();
        assert_eq!(link.kind, ChipKind::BLUETOOTH);

        // Delete
        actor.handle_delete(link_id, &mut runtime).await.unwrap();

        // Verify lookup removed
        assert!(actor.chip_pairs.get(&(params.sender, params.receiver)).is_none());
    }

    #[tokio::test]
    async fn test_handle_action() {
        use link_api::LinkAction;

        let mut actor = LinkActor::new();
        let chip_id = ChipId(1);
        let chip_kind = ChipKind::BLUETOOTH;

        // NotifyChipAdded (Global Action)
        let action = LinkAction::NotifyChipAdded(chip_id, chip_kind);

        let mut runtime = MockContext;

        // Handle Action
        actor.handle_action(None, action, &mut runtime).await.unwrap();

        // Verify chip_kind_map
        assert_eq!(actor.chip_kind_map.get(&chip_id), Some(&ChipKind::BLUETOOTH));

        // NotifyChipRemoved
        let action = LinkAction::NotifyChipRemoved(chip_id);
        actor.handle_action(None, action, &mut runtime).await.unwrap();

        // Verify chip_kind_map removed
        assert!(actor.chip_kind_map.get(&chip_id).is_none());
    }
}
