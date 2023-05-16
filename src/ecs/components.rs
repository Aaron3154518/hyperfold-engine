use std::marker::PhantomData;

use super::entities::Entity;

// Containers
pub type Container<T> = Vec<T>;

// Labels
pub type Label<T> = PhantomData<T>;
pub type AndLabels<T> = PhantomData<T>;
pub type OrLabels<T> = PhantomData<T>;
pub type NandLabels<T> = PhantomData<T>;
pub type NorLabels<T> = PhantomData<T>;

pub type Not<T> = PhantomData<T>;
pub type And<T> = PhantomData<T>;
pub type Or<T> = PhantomData<T>;

pub type Globals<G> = G;

#[derive(Debug)]
pub struct Components<'a, C, L> {
    pub eid: &'a Entity,
    pub data: C,
    pub labels: PhantomData<L>,
}

impl<'a, C, L> Components<'a, C, L> {
    pub fn new(eid: &'a Entity, data: C) -> Self {
        Self {
            eid,
            data,
            labels: PhantomData,
        }
    }
}

pub type ComponentsVec<'a, C, L> = Vec<Components<'a, C, L>>;

pub trait AddComponent<T> {
    fn add_component(&mut self, e: Entity, t: T);
}

#[macro_export]
macro_rules! add_components {
    ($cm: ident, $eid: ident, $($comps: expr),*$(,)?) => {
        $($cm.add_component($eid, $comps);)*
    };
}
