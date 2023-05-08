use std::marker::PhantomData;

use super::entities::Entity;

// Containers
pub type Components<T> = Vec<T>;

// Labels
pub type Label<T> = PhantomData<T>;
pub type AndLabels<T> = PhantomData<T>;
pub type OrLabels<T> = PhantomData<T>;
pub type NandLabels<T> = PhantomData<T>;
pub type NorLabels<T> = PhantomData<T>;

#[macro_export]
macro_rules! add_components {
    ($cm: ident, $eid: ident, $($comps: expr),*$(,)?) => {
        $($cm.add_component($eid, $comps);)*
    };
}

#[macro_export]
macro_rules! create_entity {
    ($cm: ident, $($comps: expr),*$(,)?) => {
        {
            use crate::ecs::{entities, add_components};
            let e = entities::new();
            add_components!($cm, e, $($comps),*);
            e
        }
    };
}
