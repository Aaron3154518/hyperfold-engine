use ecs_lib;
use uuid::Uuid;

pub type Entity = Uuid;

pub fn new() -> Entity {
    Entity::new_v4()
}

#[ecs_lib::component(Global)]
type EntityTrash = Vec<Entity>;
