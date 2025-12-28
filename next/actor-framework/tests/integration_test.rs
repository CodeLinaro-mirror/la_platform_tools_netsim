use actor_framework::{ActorService, DynContext, ResourceActor};
use async_trait::async_trait;
use std::collections::HashMap;

// --- Test Service ---

#[derive(Clone, Debug, PartialEq)]
struct SimpleUser {
    id: u32,
    name: String,
    is_admin: bool,
}

#[derive(Debug)]
struct SimpleUserCreate {
    name: String,
}

#[derive(Debug)]
struct SimpleUserUpdate {
    name: Option<String>,
}

#[derive(Debug)]
enum UserAction {
    PromoteToAdmin,
    #[allow(dead_code)]
    Rename(String),
}

#[derive(Debug, thiserror::Error)]
#[error("Simple user error")]
struct SimpleUserError;

#[derive(Default)]
struct UserActor {
    users: HashMap<u32, SimpleUser>,
    next_id: u32,
}

#[async_trait]
impl actor_framework::ActorLifecycle<u32> for UserActor {
    async fn on_start(&mut self, _ctx: &mut DynContext<u32>) {
        if self.next_id == 0 {
            self.next_id = 1;
        }
    }
    type Error = SimpleUserError;
}

#[async_trait]
impl ActorService for UserActor {
    type Id = u32;
    type Create = SimpleUserCreate;
    type Update = SimpleUserUpdate;
    type Action = UserAction;
    type ActionResult = bool;
    type Error = SimpleUserError;
    type Entity = SimpleUser;

    async fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        _params: Self::Create,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::Id, Self::Error> {
        let id = id.unwrap_or_else(|| {
            let id = self.next_id;
            self.next_id += 1;
            id
        });
        let user = SimpleUser { id, name: _params.name, is_admin: false };
        self.users.insert(id, user);
        Ok(id)
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        Ok(self.users.get(&id).cloned())
    }

    async fn handle_update(
        &mut self,
        id: Self::Id,
        _update: Self::Update,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::Entity, Self::Error> {
        if let Some(user) = self.users.get_mut(&id) {
            if let Some(name) = _update.name {
                user.name = name;
            }
            Ok(user.clone())
        } else {
            Err(SimpleUserError)
        }
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<(), Self::Error> {
        self.users.remove(&id);
        Ok(())
    }

    async fn handle_action(
        &mut self,
        id: Option<Self::Id>,
        action: Self::Action,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::ActionResult, Self::Error> {
        if let Some(id) = id {
            if let Some(user) = self.users.get_mut(&id) {
                match action {
                    UserAction::PromoteToAdmin => {
                        if user.is_admin {
                            Ok(false)
                        } else {
                            user.is_admin = true;
                            Ok(true)
                        }
                    }
                    UserAction::Rename(new_name) => {
                        user.name = new_name;
                        Ok(true)
                    }
                }
            } else {
                Err(SimpleUserError)
            }
        } else {
            Err(SimpleUserError) // Global actions not implemented
        }
    }

    async fn handle_list(
        &mut self,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(self.users.values().cloned().collect())
    }
}

// --- Test ---

#[tokio::test]
async fn test_framework_full_lifecycle() {
    // Start Actor
    let (actor, client) = ResourceActor::new(10);
    tokio::spawn(actor.run(UserActor::default()));

    // 1. Create
    let payload = SimpleUserCreate { name: "Alice".into() };
    let id: u32 = client.create(payload).await.unwrap();
    assert_eq!(id, 1); // First ID should be 1

    // 2. Perform Action: Promote
    let changed: bool =
        client.perform_action(Some(id.clone()), UserAction::PromoteToAdmin).await.unwrap();
    assert!(changed);

    // Verify state
    let user: SimpleUser = client.get(id.clone()).await.unwrap().unwrap();
    assert!(user.is_admin);

    // 3. Perform Action: Promote again (should return false)
    let changed_again: bool =
        client.perform_action(Some(id.clone()), UserAction::PromoteToAdmin).await.unwrap();
    assert!(!changed_again);

    // 4. Update
    // 4. Update
    let update = SimpleUserUpdate { name: Some("Bob".into()) };
    let updated_entity = client.update(id.clone(), update).await.unwrap();
    assert_eq!(updated_entity.name, "Bob");

    let updated_user = client.get(id.clone()).await.unwrap().unwrap();
    assert_eq!(updated_user.name, "Bob");

    // 5. Delete
    client.delete(id.clone()).await.unwrap();
    let deleted_user = client.get(id.clone()).await.unwrap();
    assert!(deleted_user.is_none());
}
