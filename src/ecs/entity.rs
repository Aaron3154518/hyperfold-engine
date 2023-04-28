use ecs_lib;
use std::hash::Hash;
use uuid::Uuid;

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct Entity(Uuid);

impl Entity {
    pub fn new() -> Self {
        Self { 0: Uuid::new_v4() }
    }
}

#[ecs_lib::component(Global)]
type EntityTrash = Vec<Entity>;
