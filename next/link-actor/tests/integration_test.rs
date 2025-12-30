#[path = "common/mod.rs"]
mod common;
mod propagation_test;

use common::setup;
use link_api::{LinkAction, LinkCreate};
use netsim_model::chip::ChipId;

#[tokio::test]
async fn test_create_link_succeeds() {
    let fixture = setup().await;
    let client = &fixture.client;

    let params = LinkCreate { sender: ChipId(1), receiver: ChipId(2), rssi: -50 };

    let link_id = client.create(params).await.unwrap();

    // Verify link exists
    let links = client.list().await.unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].id, link_id);
    assert_eq!(links[0].sender, ChipId(1));
    assert_eq!(links[0].receiver, ChipId(2));
}

#[tokio::test]
async fn test_create_link_fails_mismatch() {
    let fixture = setup().await;
    let client = &fixture.client;

    // Chip 1 is BLE, Chip 3 is WIFI (from common setup)
    let params = LinkCreate { sender: ChipId(1), receiver: ChipId(3), rssi: -50 };

    let result = client.create(params).await;
    assert!(result.is_err());
    // We expect InvalidParam due to kind mismatch
}

#[tokio::test]
async fn test_create_link_fails_missing() {
    let fixture = setup().await;
    let client = &fixture.client;

    // Chip 99 does not exist
    let params = LinkCreate { sender: ChipId(1), receiver: ChipId(99), rssi: -50 };

    let result = client.create(params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_link_fails_after_remove() {
    let fixture = setup().await;
    let client = &fixture.client;

    // 1. Create a valid link first (so we have an entity to send action to)
    let params = LinkCreate { sender: ChipId(1), receiver: ChipId(2), rssi: -50 };
    let link_id = client.create(params).await.unwrap();

    // 2. Notify chip 1 removed
    // We send this as a Global Action (id = None)
    client.action(None, LinkAction::NotifyChipRemoved(ChipId(1))).await.unwrap();

    // 3. Try to create another link involving Chip 1
    // Note: We use Chip 1 and Chip 2 again.
    // Even though a link exists, the *creation* check should fail because Chip 1 is gone from map.
    // (Actually, duplicate check might run first? No, validation usually runs first).
    // Let's check actor_impl.rs: on_create checks map first.
    let params_new = LinkCreate { sender: ChipId(1), receiver: ChipId(2), rssi: -60 };

    let result = client.create(params_new).await;
    assert!(result.is_err());
    // Should be "Sender chip 1 not found"
}

#[tokio::test]
async fn test_duplicate_create_fails() {
    let fixture = setup().await;
    let client = &fixture.client;

    let params = LinkCreate { sender: ChipId(1), receiver: ChipId(2), rssi: -50 };

    // First create
    client.create(params.clone()).await.unwrap();

    // Second create (duplicate)
    let result = client.create(params).await;

    // Should fail with AlreadyExists
    assert!(result.is_err());
    // TODO: Verify exact error type if possible, but for now is_err() is sufficient
    // as the only likely error here is AlreadyExists given valid params.
}

#[tokio::test]
async fn test_chip_added_lifecycle() {
    let fixture = setup().await;
    let client = &fixture.client;

    // Chip 99 does not exist yet
    let params = LinkCreate { sender: ChipId(99), receiver: ChipId(2), rssi: -50 };
    assert!(client.create(params.clone()).await.is_err());

    // This resolves the bootstrapping issue where we need to register chips before any links exist.
    client
        .action(
            None,
            LinkAction::NotifyChipAdded(ChipId(99), netsim_model::chip::ChipKind::BLUETOOTH),
        )
        .await
        .unwrap();

    // 3. Create link with Chip 99
    let params_99 = LinkCreate { sender: ChipId(99), receiver: ChipId(2), rssi: -50 };
    client.create(params_99).await.unwrap();
}

#[tokio::test]
async fn test_create_link_fails_self() {
    let fixture = setup().await;
    let client = &fixture.client;

    // Chip 1 is BLE
    let params = LinkCreate { sender: ChipId(1), receiver: ChipId(1), rssi: -50 };

    let result = client.create(params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_chip_removal_deletes_links() {
    let fixture = setup().await;
    let client = &fixture.client;

    // 1. Create a link (1 -> 2)
    let params = LinkCreate { sender: ChipId(1), receiver: ChipId(2), rssi: -50 };
    let link_id = client.create(params).await.unwrap();

    // Verify it exists
    assert_eq!(client.list().await.unwrap().len(), 1);

    // 2. Remove Chip 1
    client.action(None, LinkAction::NotifyChipRemoved(ChipId(1))).await.unwrap();

    // 3. Verify link is gone
    let links = client.list().await.unwrap();
    assert!(links.is_empty(), "Link should be deleted after chip removal");

    // 4. Verify we can't get it by ID (if we had a get method, but list is enough)
    // Also verify we can't update it
    let update_result = client.update(link_id, link_api::LinkUpdate { rssi: Some(-60) }).await;
    assert!(update_result.is_err(), "Should not be able to update deleted link");
}

#[tokio::test]
async fn test_update_propagation() {
    use netsim_model::chip::ChipKind;

    let mut fixture = setup().await;
    let client = &fixture.client;
    let mock_controller =
        fixture.mock_chip_controller.as_mut().expect("MockChipController not found in fixture");
    let mock_client =
        fixture.mock_chip_client.as_ref().expect("MockChipClient not found in fixture");

    let chip_id = ChipId(1);
    let peer_id = ChipId(2);

    // Note: setup() already adds chips 1, 2 (BT) and 3 (WIFI).
    // We don't need to notify chip added again for 1 and 2.

    // Create Link
    mock_controller.expect_update(chip_id).return_ok(netsim_model::chip::Chip::default());

    let create_params = LinkCreate { sender: chip_id, receiver: peer_id, rssi: -50 };
    client.create(create_params).await.unwrap();

    // Verify update sent to chip client
    let updates = mock_client.updates.lock().unwrap();
    // We expect 1 update for sender (1)
    assert!(updates.len() >= 1, "Expected at least 1 update, got {}", updates.len());

    // Check update for chip 1
    let update_1 = updates.iter().find(|(id, _)| *id == chip_id);
    assert!(update_1.is_some(), "Update for chip 1 not found");
    let (_, patch_1) = update_1.unwrap();
    assert!(patch_1.links.is_some());
    let links_1 = patch_1.links.as_ref().unwrap();
    assert!(links_1.contains(&(peer_id, -50)));
}
