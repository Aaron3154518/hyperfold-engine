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

pub trait AddComponent<T> {
    fn add_component(&mut self, e: Entity, t: T);
}

#[macro_export]
macro_rules! add_components {
    ($cm: ident, $eid: ident, $($comps: expr),*$(,)?) => {
        $($cm.add_component($eid, $comps);)*
    };
}
