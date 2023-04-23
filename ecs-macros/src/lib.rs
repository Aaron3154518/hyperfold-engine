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
macro_rules! service_function {
    // Does not take components
    ($cm: ident, c(), g($($g_vs: ident, $g_ts: ty),*)) => {
        |cm: &mut $cm, f: &dyn Fn($($g_ts,)*)| {
            f($(&mut cm.$g_vs,)*)
        }
    };

    ($cm: ident,
        c($($c_vs: ident, $c_ts: ty),*),
        g($($g_vs: ident, $g_ts: ty),*)
    ) => {
        |cm: &mut $cm, f: &dyn Fn($($c_ts,)*$($g_ts,)*)| {
            for key in intersect_keys(&[$(get_keys(&cm.$c_vs)),*]).iter() {
                if let ($(Some($c_vs)),*) = ($(cm.$c_vs.get_mut(key)),*) {
                    f($($c_vs,)*$(&mut cm.$g_vs,)*)
                }
            }
        }
    };
}

#[macro_export]
macro_rules! systems {
    ($sm: ident, $cm: ident, $em: ident, $c_eb: ident, $c_ce: ident,
        $(
            (
                ($($f: path [$($e_v: path, $e_i: literal),*]),+),
                c($($c_vs: ident, $c_ts: ty),*),
                g($($g_vs: ident, $g_ts: ty),*)
            )
        ),+
    ) => {
        struct $sm {
            pub component_manager: $cm,
            systems: std::collections::HashMap<crate::ecs::event::TypeIdx, Vec<Box<dyn Fn(&mut $cm)>>>,
        }

        impl $sm {
            pub fn new() -> Self {
                Self {
                    component_manager: $cm::new(),
                    systems: std::collections::HashMap::new()
                }
            }

            pub fn tick(&mut self) {
                self.component_manager.$c_eb.reset();
                while let Some(e) = self.component_manager.$c_eb.pop() {
                    self.component_manager.$c_ce = crate::ecs::event::CurrEvent::from(e);
                    let k = self.component_manager.$c_ce.get().to_idx();
                    if let Some(systems) = self.systems.get(k) {
                        for system in systems.iter() {
                            (system)(&mut self.component_manager);
                        }
                    }
                }
            }

            fn add_system<T: 'static>(&mut self, i: usize, f: Box<dyn Fn(&mut $cm)>) {
                let k = crate::ecs::event::TypeIdx::new::<T>(i);
                if let Some(v) = self.systems.get_mut(&k) {
                    v.push(f);
                } else {
                    self.systems.insert(k, vec![f]);
                }
            }

            pub fn add_systems(&mut self) {
            $(
                let f = service_function!($cm, c($($c_vs, $c_ts),*), g($($g_vs, $g_ts),*));
                $(
                    $(
                        self.add_system::<$e_v>($e_i, Box::new(move |cm: &mut $cm| f(cm, &$f)));
                    )*
                )*
            )*
            }
        }
    };
}

pub trait To<T> {
    fn to<'a>(&'a self) -> Option<&'a T>;
}

#[macro_export]
macro_rules! events {
    ($ev: ident, $($evs: ident ($ets: path)),*) => {
        #[derive(PartialEq, Eq)]
        pub enum $ev {
            $($evs($ets)),*
        }

        impl $ev {
            pub fn to_idx(&self) -> &'static crate::ecs::event::TypeIdx {
                match self {
                    $(Self::$evs(v) => v.to_idx(),)*
                }
            }
        }

        $(
            impl From<$ets> for $ev {
                fn from(v: $ets) -> Self {
                    Self::$evs(v)
                }
            }

            impl To<$ets> for $ev {
                fn to<'a>(&'a self) -> Option<&'a $ets> {
                    match self {
                        Self::$evs(v) => Some(v),
                        _ => None
                    }
                }
            }
        )*
    }
}

#[macro_export]
macro_rules! match_event {
    ($ev: ident, $e: ident :: $v: ident $(($($vs: ident),+))?, $body: block) => {
        if let Some(e) = $ev.get().to() {
            if let $e::$v$(($($vs),*))? = e {
                $body
            }
        }
    };
}
