use ecs_lib;
use uuid::Uuid;

pub type Entity = Uuid;

pub fn new() -> Entity {
    Entity::new_v4()
}

#[ecs_lib::global]
struct EntityTrash(pub Vec<Entity>);

impl EntityTrash {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}
