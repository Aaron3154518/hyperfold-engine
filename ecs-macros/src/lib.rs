use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

pub mod structs;

pub trait ComponentManager<E, T> {
    fn add_component(&mut self, e: E, t: T);
}

#[macro_export]
macro_rules! manager {
    ($cm: ident, c($($c_v: ident, $c_t: ty),*), g($($g_v: ident, $g_t: ty),*)) => {
        pub struct $cm {
            $($g_v: $g_t,)*
            $($c_v: std::collections::HashMap<crate::ecs::entity::Entity, $c_t>,)*
        }

        impl $cm {
            pub fn new() -> Self {
                Self {
                    $($g_v: <$g_t>::new(),)*
                    $($c_v: std::collections::HashMap::new(),)*
                }
            }
        }

        $(
            impl ComponentManager<crate::ecs::entity::Entity, $c_t> for $cm {
                fn add_component(&mut self, e: crate::ecs::entity::Entity, t: $c_t) {
                    self.$c_v.insert(e, t);
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
macro_rules! test {
    (i: ident) => {
        fn $i() {}
    };
}

#[macro_export]
macro_rules! systems {
    ($sm: ident, $cm: ident,
        $(
            (
                ($($f: path),+),
                c($($c_vs: ident, $c_ts: ty),*),
                g($($g_vs: ident, $g_ts: ty),*)
            )
        ),+
    ) => {
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

            pub fn add_systems(&mut self) {
            $(
                let f = |cm: &mut $cm, f: &dyn Fn($($c_ts),*,$($g_ts),*)| {
                    for key in intersect_keys(&[$(get_keys(&cm.$c_vs)),*]).iter() {
                        if let ($(Some($c_vs)),*) = ($(cm.$c_vs.get_mut(key)),*) {
                            f($($c_vs),*,$(&mut cm.$g_vs),*)
                        }
                    }
                };
                $(
                    self.systems.push(Box::new(move |cm: &mut $cm| f(cm, &$f)));
                )*
            )*
            }
        }
    };
}

// struct C;
// struct C2;

// manager!(Foo, c0, C, c1, C2);
// systems!(SFoo, Foo, (c0, &C, c1, &mut C2));
