use std::marker::PhantomData;

use ecs_lib::{component, event, global};

use super::entity::Entity;

pub type Components<T> = Vec<T>;
pub type Label<T> = PhantomData<T>;
pub type AndLabels<T> = PhantomData<T>;
pub type OrLabels<T> = PhantomData<T>;
pub type NandLabels<T> = PhantomData<T>;
pub type NorLabels<T> = PhantomData<T>;

pub trait LabelTrait {
    fn add_label(&self, cm: &mut crate::CFoo, eid: Entity);
}

#[component]
pub struct Component {
    pub name: &'static str,
    pub loc: &'static str,
}

#[component]
pub struct MyComponent {
    pub msg: String,
}

#[global]
pub struct Resource {
    pub cnt: i32,
}

impl Resource {
    pub fn new() -> Self {
        Self { cnt: 0 }
    }
}

#[event]
pub enum OtherEvent {
    O1,
    O2(String, i32),
    O3,
}
