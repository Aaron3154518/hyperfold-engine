use std::marker::PhantomData;

use super::entity::Entity;

// Containers
pub type Components<T> = Vec<T>;

// Labels
pub type Label<T> = PhantomData<T>;
pub type AndLabels<T> = PhantomData<T>;
pub type OrLabels<T> = PhantomData<T>;
pub type NandLabels<T> = PhantomData<T>;
pub type NorLabels<T> = PhantomData<T>;

pub trait LabelTrait {
    fn add_label(&self, cm: &mut crate::CFoo, eid: Entity);
}
