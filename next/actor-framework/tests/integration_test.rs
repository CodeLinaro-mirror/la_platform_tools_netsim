use actor_framework::{ActorEntity, ResourceActor, Runtime};
use async_trait::async_trait;

// --- Test Entity ---

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

#[async_trait]
impl ActorEntity for SimpleUser {
    type Id = u32;
    type Create = SimpleUserCreate;
    type Update = SimpleUserUpdate;
    type Action = UserAction;
    type ActionResult = bool;
    type Context = ();
    type Error = SimpleUserError;
    type ListResponse = Vec<SimpleUser>;

    fn from_create_params(id: u32, params: SimpleUserCreate) -> Result<Self, Self::Error> {
        Ok(Self { id, name: params.name, is_admin: false })
    }

    fn on_list(
        entities: &std::collections::HashMap<Self::Id, Self>,
        _context: &mut Self::Context,
        _runtime: &mut impl Runtime,
    ) -> Self::ListResponse {
        entities.values().cloned().collect()
    }

    async fn on_update(
        &mut self,
        update: SimpleUserUpdate,
        _context: &mut Self::Context,
        _runtime: &mut impl Runtime,
    ) -> Result<(), Self::Error> {
        if let Some(name) = update.name {
            self.name = name;
        }
        Ok(())
    }

    async fn on_delete(
        &self,
        _context: &mut Self::Context,
        _runtime: &mut impl Runtime,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn handle_action(
        &mut self,
        _action: Self::Action,
        _context: &mut Self::Context,
        _runtime: &mut impl Runtime,
    ) -> Result<Self::ActionResult, Self::Error> {
        match _action {
            UserAction::PromoteToAdmin => {
                if self.is_admin {
                    Ok(false)
                } else {
                    self.is_admin = true;
                    Ok(true)
                }
            }
            UserAction::Rename(new_name) => {
                self.name = new_name;
                Ok(true)
            }
        }
    }
}

// --- Test ---

#[tokio::test]
async fn test_framework_full_lifecycle() {
    // Start Actor
    let (actor, client) = ResourceActor::new(10);
    tokio::spawn(actor.run(()));

    // 1. Create
    let payload = SimpleUserCreate { name: "Alice".into() };
    let id: u32 = client.create(payload).await.unwrap();
    assert_eq!(id, 1); // First ID should be 1

    // 2. Perform Action: Promote
    let changed: bool =
        client.perform_action(id.clone(), UserAction::PromoteToAdmin).await.unwrap();
    assert!(changed);

    // Verify state
    let user: SimpleUser = client.get(id.clone()).await.unwrap().unwrap();
    assert!(user.is_admin);

    // 3. Perform Action: Promote again (should return false)
    let changed_again: bool =
        client.perform_action(id.clone(), UserAction::PromoteToAdmin).await.unwrap();
    assert!(!changed_again);

    // 4. Update
    let update = SimpleUserUpdate { name: Some("Bob".into()) };
    let updated_user = client.update(id.clone(), update).await.unwrap();
    assert_eq!(updated_user.name, "Bob");

    // 5. Delete
    client.delete(id.clone()).await.unwrap();
    let deleted_user = client.get(id.clone()).await.unwrap();
    assert!(deleted_user.is_none());
}
