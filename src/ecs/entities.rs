use std::collections::{HashMap, HashSet};

use uuid::Uuid;

pub trait NewEntity {
    fn new() -> Entity;
}

pub type Entity = Uuid;

impl NewEntity for Entity {
    fn new() -> Entity {
        Self::new_v4()
    }
}

pub type EntitySet = HashSet<Entity>;
pub type EntityMap<T> = HashMap<Entity, T>;

#[macros::global]
pub struct EntityTrash(pub Vec<Entity>);

impl EntityTrash {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}
