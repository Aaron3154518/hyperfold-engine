use std::any::type_name;

use super::entities::Entity;

#[derive(Debug)]
pub enum Singleton<V> {
    Some { e: Entity, v: V },
    None,
}

impl<V> Singleton<V> {
    pub fn new(e: Entity, v: V) -> Self {
        Self::Some { e, v }
    }

    pub fn typename(&self) -> &'static str {
        type_name::<V>()
    }

    // Returns true if successfully set (no existing value)
    pub fn set(&mut self, other: &mut Self) -> bool {
        let is_some = matches!(self, Singleton::Some { .. });
        let other_is_some = matches!(other, Singleton::Some { .. });
        match (is_some, other_is_some) {
            (true, true) => false,
            (true, false) => true,
            (false, true) => {
                *self = std::mem::replace(other, Singleton::None);
                true
            }
            (false, false) => true,
        }
    }

    pub fn remove(&mut self, key: &Entity) {
        if matches!(self, Singleton::Some { e, .. } if e == key) {
            *self = Singleton::None;
        }
    }

    pub fn contains_key(&self, key: &Entity) -> bool {
        self.get_key().is_some_and(|e| e == key)
    }

    pub fn get_key(&self) -> Option<&Entity> {
        match self {
            Singleton::Some { e, .. } => Some(e),
            Singleton::None => None,
        }
    }

    pub fn get_value<'a>(&'a self, key: &Entity) -> Option<&'a V> {
        match self {
            Singleton::Some { e, v } => (e == key).then_some(v),
            Singleton::None => None,
        }
    }

    pub fn get_value_mut<'a>(&'a mut self, key: &Entity) -> Option<&'a mut V> {
        match self {
            Singleton::Some { e, v } => (e == key).then_some(v),
            Singleton::None => None,
        }
    }

    pub fn get(&self) -> Option<(&Entity, &V)> {
        match self {
            Singleton::Some { e, v } => Some((e, v)),
            Singleton::None => None,
        }
    }

    pub fn get_mut(&mut self) -> Option<(&Entity, &mut V)> {
        match self {
            Singleton::Some { e, v } => Some((e, v)),
            Singleton::None => None,
        }
    }

    pub fn get_vec(&self) -> Vec<(&Entity, &V)> {
        match self {
            Singleton::Some { e, v } => vec![(e, v)],
            Singleton::None => vec![],
        }
    }

    pub fn get_vec_mut(&mut self) -> Vec<(&mut Entity, &mut V)> {
        match self {
            Singleton::Some { e, v } => vec![(e, v)],
            Singleton::None => vec![],
        }
    }
}

pub trait AddComponent<T> {
    fn add_component(&mut self, e: Entity, t: T);
}

#[macro_export]
macro_rules! add_components {
    ($cm: ident, $eid: ident, $($comps: expr),*$(,)?) => {
        $($cm.add_component($eid, $comps);)*
    };
}
