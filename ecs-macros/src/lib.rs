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
    ($cm: ident,
        c($($c_v: ident, $c_t: ty),*),
        g($($g_v: ident, $g_t: ty),*)) => {
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
macro_rules! service_function {
    // Does not take components
    ($cm: ident,
        e($e_t: path),
        c(),
        g($($g_vs: ident, $g_ts: ty),*)) => {
        |cm: &mut $cm, e: &$e_t, f: &dyn Fn(&$e_t, $($g_ts,)*)| {
            f(e, $(&mut cm.$g_vs,)*)
        };
    };

    ($cm: ident,
        e($e_t: path),
        c($($c_vs: ident, $c_ts: ty),*),
        g($($g_vs: ident, $g_ts: ty),*)
    ) => {
        |cm: &mut $cm, e: &$e_t, f: &dyn Fn(&$e_t, $($c_ts,)*$($g_ts,)*)| {
            for key in intersect_keys(&[$(get_keys(&cm.$c_vs)),*]).iter() {
                if let ($(Some($c_vs)),*) = ($(cm.$c_vs.get_mut(key)),*) {
                    f(e, $($c_vs,)*$(&mut cm.$g_vs,)*)
                }
            }
        };
    };
}

#[macro_export]
macro_rules! systems {
    ($sm: ident, $cm: ident, $em: ident, $c_eb: ident,
        $(
            (
                ($($f: path),+),
                e($e_v: path),
                c($($c_vs: ident, $c_ts: ty),*),
                g($($g_vs: ident, $g_ts: ty),*)
            )
        ),+
    ) => {
        use crate::ecs::event::{PushEvent, PopRun};

        struct $sm {
            pub component_manager: $cm,
        }

        impl $sm {
            pub fn new() -> Self {
                Self {
                    component_manager: $cm::new(),
                }
            }

            pub fn tick(&mut self) {
                self.component_manager.$c_eb.reset();
                while self.component_manager.$c_eb.pop_run(
                    &mut self.component_manager) {
                }
            }

            pub fn add_systems(&mut self) {
            $(
                let f = service_function!($cm,
                    e($e_v),
                    c($($c_vs, $c_ts),*),
                    g($($g_vs, $g_ts),*));
                $(
                    self.component_manager.$c_eb.add_system(Box::new(move |cm: &mut $cm, e: &$e_v| f(cm, e, &$f)));
                )*
            )*
            }
        }
    };
}

#[macro_export]
macro_rules! systems_struct {
    ($sm: ident, $($evs: ident :$ets: path),*) => {
        pub struct $sm {
            pub: order: Vec<VecDeque<$ets>>,
            $(pub $evs: Vec<$ets>),*
            // $(pub $evs: Vec<Box<dyn Fn(&mut crate::Foo, &$ets)>>),*
        }

        impl $sm {
            pub fn new() -> Self {
                Self {
                    $($evs: Vec::new()),*
                }
            }

            pub fn extend(&mut self, other: &mut Self) {
                $(self.$evs.;)*
            }
        }
    }
}

#[macro_export]
macro_rules! event_impl {
    ($name: ident, $var: ident) => {
        impl crate::ecs::event::PushEvent<$name> for crate::ecs::event::EventBus {
            fn push(&mut self, e: $name) {
                if let Some(queue) = self.stack.last_mut() {
                    queue.push_back(Box::new(crate::ecs::event::RunEvent {
                        e,
                        funcs: self.manager.$var.iter().map(|b| b).collect(),
                    }));
                }
            }

            fn add_system(&mut self, f: Box<dyn Fn(&mut crate::Foo, &$name)>) {
                self.manager.$var.push(f);
            }
        }
    };
}
