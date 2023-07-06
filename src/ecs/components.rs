use std::marker::PhantomData;

use crate::intersect::HasKey;

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

#[derive(Debug)]
pub enum Singleton<K, V> {
    Some { k: K, v: V },
    None,
}

impl<K, V> Singleton<K, V>
where
    K: PartialEq,
{
    pub fn new(k: K, v: V) -> Self {
        Self::Some { k, v }
    }

    // Returns true if successfully set (no existing value)
    pub fn set(&mut self, other: Self) -> bool {
        match other {
            Singleton::Some { k, v } => {
                let is_some = match self {
                    Singleton::Some { .. } => true,
                    Singleton::None => false,
                };
                if !is_some {
                    *self = Singleton::Some { k, v };
                }
                !is_some
            }
            Singleton::None => true,
        }
    }

    pub fn remove(&mut self, key: &K) {
        if matches!(self, Singleton::Some { k, .. } if k == key) {
            *self = Singleton::None;
        }
    }

    pub fn get_key(&self) -> Option<K>
    where
        K: Copy,
    {
        match self {
            Singleton::Some { k, .. } => Some(*k),
            Singleton::None => None,
        }
    }

    pub fn get<'a>(&'a self, key: &K) -> Option<&'a V> {
        match self {
            Singleton::Some { k, v } => (k == key).then_some(v),
            Singleton::None => None,
        }
    }

    pub fn get_mut<'a>(&'a mut self, key: &K) -> Option<&'a mut V> {
        match self {
            Singleton::Some { k, v } => (k == key).then_some(v),
            Singleton::None => None,
        }
    }
}

impl<K, V> HasKey<K> for Singleton<K, V>
where
    K: PartialEq,
{
    fn has_key(&self, key: &K) -> bool {
        matches!(self, Singleton::Some { k, .. } if k == key)
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
