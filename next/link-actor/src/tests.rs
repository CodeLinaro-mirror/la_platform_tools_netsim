// Copyright 2025 The Android Open Source Project

#[cfg(test)]
mod tests {
    use crate::entity::{LinkContext, LinkEntity};
    use actor_framework::{ActorEntity, BoxStream, Runtime};
    use link_api::{LinkCreate, LinkId};
    use netsim_model::chip::{ChipId, ChipKind};
    use std::time::Duration;

    struct MockRuntime;

    impl Runtime for MockRuntime {
        fn set_interval(&mut self, _duration: Duration) {}
        fn add_stream(&mut self, _id: usize, _stream: BoxStream) {}
        fn shutdown(&mut self) {}
    }

    #[tokio::test]
    async fn test_link_entity_lifecycle() {
        let mut ctx = LinkContext::default();
        let sender = ChipId(1);
        let receiver = ChipId(2);

        // Pre-populate chip_kind_map to test kind inference
        ctx.chip_kind_map.insert(sender, ChipKind::BLUETOOTH);
        ctx.chip_kind_map.insert(receiver, ChipKind::BLUETOOTH);

        let link_id = LinkId(1);
        let params = LinkCreate { sender, receiver, rssi: -50 };

        let mut runtime = MockRuntime;

        // Create
        let mut entity = LinkEntity::from_create_params(link_id, params.clone()).unwrap();
        entity.on_create(&mut ctx, &mut runtime).await.unwrap();

        // Verify lookup
        assert_eq!(ctx.lookup.get(&(params.sender, params.receiver)), Some(&link_id));

        // Verify kind set from params
        assert_eq!(entity.link.kind, ChipKind::BLUETOOTH);

        // Delete
        entity.on_delete(&mut ctx, &mut runtime).await.unwrap();

        // Verify lookup removed
        assert!(ctx.lookup.get(&(params.sender, params.receiver)).is_none());
    }

    #[tokio::test]
    async fn test_handle_action() {
        use link_api::LinkAction;

        let mut ctx = LinkContext::default();
        let chip_id = ChipId(1);
        let chip_kind = ChipKind::BLUETOOTH;

        // NotifyChipAdded
        let action = LinkAction::NotifyChipAdded(chip_id, chip_kind);

        let link_id = LinkId(1);
        let params = LinkCreate { sender: ChipId(1), receiver: ChipId(2), rssi: -50 };
        let mut entity = LinkEntity::from_create_params(link_id, params).unwrap();

        let mut runtime = MockRuntime;

        entity.handle_action(action, &mut ctx, &mut runtime).await.unwrap();

        // Verify chip_kind_map
        assert_eq!(ctx.chip_kind_map.get(&chip_id), Some(&ChipKind::BLUETOOTH));

        // NotifyChipRemoved
        let action = LinkAction::NotifyChipRemoved(chip_id);
        entity.handle_action(action, &mut ctx, &mut runtime).await.unwrap();

        // Verify chip_kind_map removed
        assert!(ctx.chip_kind_map.get(&chip_id).is_none());
    }
}
