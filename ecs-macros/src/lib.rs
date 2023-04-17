use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

pub trait ComponentManager<E, T> {
    fn add_component(&mut self, e: E, t: T);
}

#[macro_export]
macro_rules! manager {
    ($cm: ident, $($v: ident, $t: ty),*) => {
        pub struct $cm {
            $($v:  std::collections::HashMap<crate::ecs::entity::Entity, $t>),*
        }

        impl $cm {
            pub fn new() -> Self {
                Self {
                    $($v: std::collections::HashMap::new()),*
                }
            }
        }

        $(
            impl ComponentManager<crate::ecs::entity::Entity, $t> for $cm {
                fn add_component(&mut self, e: crate::ecs::entity::Entity, t: $t) {
                    self.$v.insert(e, t);
                }
            }
        )*
    };
}

pub trait ComponentSystems<F> {
    fn add_system(&mut self, f: F);
}

pub fn intersect_keys<K: Eq + Hash + Clone>(keys: &[HashSet<&K>]) -> HashSet<K> {
    if let Some(k1) = keys.first() {
        let mut k1 = k1.clone();
        keys[1..]
            .iter()
            .for_each(|k| k1 = k1.intersection(k).map(|k| *k).collect::<HashSet<_>>());
        return k1.iter().map(|k| (*k).clone()).collect();
    }
    HashSet::new()
}

pub fn get_keys<'a, K: Eq + Hash + Clone, V>(map: &'a HashMap<K, V>) -> HashSet<&'a K> {
    map.keys().collect()
}

#[macro_export]
macro_rules! systems {
    ($sm: ident, $cm: ident, $(($($vs: ident, $ts: ty),+)),+) => {
        struct $sm {
            pub component_manager: $cm,
            systems: Vec<Box<dyn Fn(&mut $cm)>>,
        }

        impl $sm {
            pub fn new() -> Self {
                Self {
                    component_manager: $cm::new(),
                    systems: Vec::new()
                }
            }

            pub fn tick(&mut self) {
                for system in self.systems.iter() {
                    (system)(&mut self.component_manager);
                }
            }
        }

        $(
            impl ComponentSystems<&'static dyn Fn($($ts),*)> for $sm {
                fn add_system(&mut self, f: &'static dyn Fn($($ts),*)) {
                    self.systems.push(
                        Box::new(|cm: &mut $cm| {
                            for key in intersect_keys(&[$(get_keys(&cm.$vs)),*]).iter() {
                                if let ($(Some($vs)),*) = ($(cm.$vs.get_mut(key)),*) {
                                    (f)($($vs),*)
                                }
                            }
                        })
                    );
                }
            }
        )*
    };
}

// struct C;
// struct C2;

// manager!(Foo, c0, C, c1, C2);
// systems!(SFoo, Foo, (c0, &C, c1, &mut C2));
