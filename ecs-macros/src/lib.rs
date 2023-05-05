#![feature(hash_drain_filter)]

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    hash::Hash,
};

pub mod structs;

pub trait Mut<T> {
    fn new_event(&mut self, t: T);

    fn get_event<'a>(&'a self) -> Option<&'a T>;
}

#[macro_export]
macro_rules! events {
    ($em: ident, $(($s: path, $e: ident, $v: ident)),*) => {
        #[derive(Hash, Clone, Copy, Eq, PartialEq, Debug)]
        pub enum E {
            $($e),*
        }

        #[derive(Debug)]
        pub struct $em {
            $($v: Vec<$s>),*,
            events: VecDeque<(E, usize)>
        }

        impl $em {
            pub fn new() -> Self {
                Self {
                    $($v: Vec::new()),*,
                    events: VecDeque::new()
                }
            }

            pub fn has_events(&self) -> bool {
                !self.events.is_empty()
            }

            fn add_event(&mut self, e: E) {
                self.events.push_back((e, 0));
            }

            pub fn get_events(&mut self) -> VecDeque<(E, usize)> {
                std::mem::replace(&mut self.events, VecDeque::new())
            }

            pub fn append(&mut self, other: &mut Self) {
                $(other.$v.reverse();self.$v.append(&mut other.$v);)*
            }

            pub fn pop(&mut self, e: E) {
                match e {
                    $(
                        E::$e => {
                            self.$v.pop();
                        }
                    )*
                }
            }
        }

        $(
            impl Mut<$s> for $em {
                fn new_event(&mut self, t: $s) {
                    self.$v.push(t);
                    self.add_event(E::$e);
                }

                fn get_event<'a>(&'a self) -> Option<&'a $s> {
                    self.$v.last()
                }
            }
        )*
    }
}

pub trait ComponentManager<E, T> {
    fn add_component(&mut self, e: E, t: T);
}

#[macro_export]
macro_rules! c_manager {
    ($cm: ident, $($c_v: ident, $c_t: ty),*) => {
        pub struct $cm {
            eids: std::collections::HashSet<crate::ecs::entity::Entity>,
            $($c_v: std::collections::HashMap<crate::ecs::entity::Entity, $c_t>,)*
        }

        impl $cm {
            pub fn new() -> Self {
                Self {
                    eids: std::collections::HashSet::new(),
                    $($c_v: std::collections::HashMap::new(),)*
                }
            }

            pub fn append(&mut self, cm: &mut $cm) {
                self.eids.extend(cm.eids.drain());
                $(self.$c_v.extend(cm.$c_v.drain());)*
            }

            pub fn remove(&mut self, tr: &mut crate::ecs::entity::EntityTrash) {
                for eid in tr.0.drain(..) {
                    self.eids.remove(&eid);
                    $(self.$c_v.remove(&eid);)*
                }
            }

            pub fn add_labels(&mut self, e: crate::ecs::entity::Entity, ls: Vec<&dyn crate::ecs::component::LabelTrait>) {
                for l in ls {
                    l.add_label(self, e);
                }
            }

            pub fn add_label(&mut self, e: crate::ecs::entity::Entity, l: impl crate::ecs::component::LabelTrait) {
                l.add_label(self, e);
            }
        }

        $(
            impl ComponentManager<crate::ecs::entity::Entity, $c_t> for $cm {
                fn add_component(&mut self, e: crate::ecs::entity::Entity, t: $c_t) {
                    self.eids.insert(e);
                    self.$c_v.insert(e, t);
                }
            }
        )*
    };
}

#[macro_export]
macro_rules! g_manager {
    ($gm: ident, $($g_v: ident, $g_t: ty),*) => {
        pub struct $gm {
            $($g_v: $g_t,)*
        }

        impl $gm {
            pub fn new() -> Self {
                Self {
                    $($g_v: <$g_t>::new(),)*
                }
            }
        }
    };
}

pub trait ComponentSystems<F> {
    fn add_system(&mut self, f: F);
}

pub fn intersect<'a, K, V1, V2, F>(
    mut h: HashMap<&'a K, V1>,
    h_new: &'a HashMap<K, V2>,
    get: F,
) -> HashMap<&'a K, V1>
where
    K: Eq + Hash + Clone + Ord,
    F: Fn(&mut V1) -> &mut Option<&'a V2>,
{
    for (k, v) in h_new.iter() {
        if let Some(v2) = h.get_mut(k) {
            *get(v2) = Some(v)
        }
    }
    h.drain_filter(|_k, v| get(v).is_none());
    h
}

pub fn intersect_mut<'a, K, V1, V2, F>(
    mut h: HashMap<&'a K, V1>,
    h_new: &'a mut HashMap<K, V2>,
    get: F,
) -> HashMap<&'a K, V1>
where
    K: Eq + Hash + Clone + Ord,
    F: Fn(&mut V1) -> &mut Option<&'a mut V2>,
{
    for (k, v) in h_new.iter_mut() {
        if let Some(v2) = h.get_mut(k) {
            *get(v2) = Some(v)
        }
    }
    h.drain_filter(|_k, v| get(v).is_none());
    h
}

pub fn intersect_keys<K: Eq + Hash + Clone + Ord>(keys: &mut [HashSet<&K>]) -> BTreeSet<K> {
    keys.sort_by(|s1, s2| s1.len().cmp(&s2.len()));
    if let Some(k1) = keys.first() {
        let mut k1 = k1.clone();
        keys[1..]
            .iter()
            .for_each(|k| k1 = k1.intersection(k).map(|k| *k).collect::<HashSet<_>>());
        return k1.iter().map(|k| (*k).clone()).collect();
    }
    BTreeSet::new()
}

pub fn get_keys<'a, K: Eq + Hash + Clone, V>(map: &'a HashMap<K, V>) -> HashSet<&'a K> {
    map.keys().collect()
}

#[macro_export]
macro_rules! systems {
    ($sm: ident, $cm: ident, $gm: ident, $em: ident,
        $g_eb: ident, $g_ev: ident, $g_rs: ident, $g_cm: ident, $g_tr: ident,
        $($e_v: ident, $fs: tt),*
    ) => {
        struct $sm {
            pub gm: $gm,
            pub cm: $cm,
            stack: Vec<std::collections::VecDeque<(E, usize)>>,
            services: std::collections::HashMap<E, Vec<Box<dyn Fn(&mut $cm, &mut $gm, &mut $em)>>>,
            // Stores events in the order that they should be handled
            events: $em,
        }

        impl $sm {
            pub fn new() -> Self {
                let mut s = Self {
                    gm: $gm::new(),
                    cm: $cm::new(),
                    stack: Vec::new(),
                    services: std::collections::HashMap::new(),
                    events: $em::new()
                };
                s.add_systems();
                s
            }

            pub fn quit(&self) -> bool {
                self.gm.$g_ev.quit
            }

            pub fn get_rs<'a>(&'a mut self) -> &'a mut crate::asset_manager::RenderSystem {
                &mut self.gm.$g_rs
            }

            fn init(&mut self, ts: u32) -> $em {
                let mut events = $em::new();
                events.new_event(crate::ecs::event::CoreEvent::Events);
                events.new_event(crate::ecs::event::CoreEvent::Update(ts));
                events.new_event(crate::ecs::event::CoreEvent::Render);
                events
            }

            pub fn tick(&mut self, ts: u32, camera: &Rect, screen: &Dimensions) {
                // Update events
                self.gm.$g_ev.update(ts, camera, screen);
                // Clear the screen
                self.gm.$g_rs.r.clear();
                // Add initial events
                let mut events = self.init(ts);
                if events.has_events() {
                    self.events.append(&mut events);
                    self.stack.push(events.get_events());
                }
                while !self.stack.is_empty() {
                    // Get element from next queue
                    if let Some((e, i, n)) = self
                        .stack
                        // Get last queue
                        .last_mut()
                        // Get next events
                        .and_then(|queue| queue.front_mut())
                        // Check if the system exists
                        .and_then(|(e, i)| {
                            self.services.get(e).and_then(|v_s| {
                                // Increment the event idx and return the old values
                                v_s.get(*i).map(|_| {
                                    let vals = (e.clone(), i.clone(), v_s.len());
                                    *i += 1;
                                    vals
                                })
                            })
                        })
                    {
                        // This is the last system for this event
                        if i + 1 >= n {
                            self.pop();
                        }
                        // Add a new queue for new events
                        self.gm.$g_eb = $em::new();
                        // Run the system
                        if let Some(s) = self.services.get(&e).and_then(|v_s| v_s.get(i)) {
                            (s)(&mut self.cm, &mut self.gm, &mut self.events);
                        }
                        // If this is the last system, remove the event
                        if i + 1 >= n {
                            self.events.pop(e);
                        }
                        // Add new events
                        if self.gm.$g_eb.has_events() {
                            self.events.append(&mut self.gm.$g_eb);
                            self.stack.push(self.gm.$g_eb.get_events());
                        }
                    } else {
                        // We're done with this event
                        self.pop();
                    }
                }
                // Display the screen
                self.gm.$g_rs.r.present();
                // Remove marked entities
                self.cm.remove(&mut self.gm.$g_tr);
                // Add new entities
                self.cm.append(&mut self.gm.$g_cm);
            }

            fn pop(&mut self) {
                // Remove top element and empty queue
                if self.stack.last_mut().is_some_and(|queue| {
                    queue.pop_front();
                    queue.is_empty()
                }) {
                    self.stack.pop();
                }
            }

            fn add_system(&mut self, e: E, f: Box<dyn Fn(&mut $cm, &mut $gm, &mut $em)>) {
                if let Some(v) = self.services.get_mut(&e) {
                    v.push(f);
                } else {
                    self.services.insert(e, vec![f]);
                }
            }

            fn add_systems(&mut self) {
                $(
                    let (f) = $fs;
                    self.add_system(E::$e_v, Box::new(f));
                )*
            }
        }
    };
}
