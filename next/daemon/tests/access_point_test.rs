// Copyright 2025 The Android Open Source Project

use netsim_proto::{
    access_point::{
        AccessPoint, CreateAccessPointRequest, DeleteAccessPointRequest, DisconnectRequest,
        ExecuteAccessPointRequest, GetAccessPointRequest, ListAccessPointsRequest,
        UpdateAccessPointRequest,
    },
    protobuf,
};

use crate::world::World;

// Scenario: Access Point Lifecycle
//   Given a running Netsim Daemon
//   When I create a new Access Point
//   Then the AP exists and can be retrieved
//   When I update the AP
//   Then the AP reflects the changes
//   When I list APs
//   Then the AP is in the list
//   When I delete the AP
//   Then the AP is no longer found
#[tokio::test]
async fn test_access_point_lifecycle() {
    // Given a running Netsim Daemon
    let mut world = World::new().await;
    let _daemon_task = world.spawn_daemon();

    // Ensure client connects
    let client = world.ensure_access_point_client();

    // 1. Create AP
    let mut ap_config = AccessPoint::new();
    ap_config.ssid = "TestAP".to_string();
    ap_config.channel = 36;
    ap_config.hw_mode = "a".to_string(); // 5GHz

    let mut create_req = CreateAccessPointRequest::new();
    create_req.access_point = protobuf::MessageField::some(ap_config);

    let created_ap =
        client.create_async(&create_req).expect("Create AP failed").await.expect("RPC failed");

    let ap_id = created_ap.id;
    assert!(ap_id > 0);
    assert_eq!(created_ap.ssid, "TestAP");
    assert_eq!(created_ap.channel, 36);
    assert_eq!(created_ap.hw_mode, "a");

    // 2. Get AP
    let mut get_req = GetAccessPointRequest::new();
    get_req.id = ap_id;
    let fetched_ap = client.get_async(&get_req).expect("Get AP failed").await.expect("RPC failed");

    assert_eq!(fetched_ap.id, ap_id);
    assert_eq!(fetched_ap.ssid, "TestAP");

    // 3. Update AP
    let mut update_req = UpdateAccessPointRequest::new();
    update_req.id = ap_id;
    update_req.ssid = Some("UpdatedAP".to_string());
    update_req.channel = Some(40);

    let updated_ap =
        client.update_async(&update_req).expect("Update AP failed").await.expect("RPC failed");

    assert_eq!(updated_ap.ssid, "UpdatedAP");
    assert_eq!(updated_ap.channel, 40);

    // Verify update with Get
    let fetched_updated_ap =
        client.get_async(&get_req).expect("Get AP failed").await.expect("RPC failed");
    assert_eq!(fetched_updated_ap.ssid, "UpdatedAP");

    // 4. List APs
    let list_req = ListAccessPointsRequest::new();
    let list_resp =
        client.list_async(&list_req).expect("List APs failed").await.expect("RPC failed");

    assert!(list_resp.access_points.iter().any(|ap| ap.id == ap_id));

    // 5. Execute Disconnect (Action)
    let mut disconnect_req = DisconnectRequest::new();
    disconnect_req.mac_address = "00:11:22:33:44:55".to_string();

    let mut execute_req = ExecuteAccessPointRequest::new();
    execute_req.id = ap_id;
    execute_req.set_disconnect(disconnect_req);

    client.execute_async(&execute_req).expect("Execute failed").await.expect("RPC failed");

    // 6. Delete AP
    let mut delete_req = DeleteAccessPointRequest::new();
    delete_req.id = ap_id;
    client.delete_async(&delete_req).expect("Delete AP failed").await.expect("RPC failed");

    // Verify Deletion (Get should fail)
    let get_result = client.get_async(&get_req).expect("Get AP failed").await;
    assert!(get_result.is_err(), "Get should fail after deletion");
}
