use std::{
    collections::{HashMap, HashSet},
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
macro_rules! function_body {
    // Does not take components
    ($cm: ident, $f: ident, $e: ident,
        c(),
        g($($g_vs: ident, $g_ts: ty),*)) => {
        $f($e, $(&mut $cm.$g_vs,)*)
    };

    ($cm: ident, $f: ident, $e: ident,
        c($($c_vs: ident, $c_ts: ty),*),
        g($($g_vs: ident, $g_ts: ty),*)
    ) => {
        for key in intersect_keys(&[$(get_keys(&$cm.$c_vs)),*]).iter() {
            if let ($(Some($c_vs)),*) = ($($cm.$c_vs.get_mut(key)),*) {
                $f($e, $($c_vs,)*$(&mut $cm.$g_vs,)*)
            }
        }
    };
}

#[macro_export]
macro_rules! systems {
    ($sm: ident, $cm: ident, $em: ident, $c_eb: ident,
        $(
            (
                ($($f: path),+),
                e($e_t: path, $e_v: ident),
                c($($c_vs: ident, $c_ts: ty),*),
                g($($g_vs: ident, $g_ts: ty),*)
            )
        ),+
    ) => {
        struct $sm {
            pub cm: $cm,
            stack: Vec<std::collections::VecDeque<(E, usize)>>,
            services: std::collections::HashMap<E, Vec<Box<dyn Fn(&mut $cm, &mut $em)>>>,
            // Stores events in the order that they should be handled
            events: $em,
        }

        impl $sm {
            pub fn new() -> Self {
                Self {
                    cm: $cm::new(),
                    stack: Vec::new(),
                    services: std::collections::HashMap::new(),
                    events: $em::new()
                }
            }

            fn init(&mut self) -> $em {
                let mut events = $em::new();
                events.new_event(crate::ecs::event::CoreEvent::Update);
                events.new_event(crate::ecs::event::CoreEvent::Events);
                events.new_event(crate::ecs::event::CoreEvent::Render);
                events
            }

            pub fn tick(&mut self) {
                let mut events = self.init();
                if events.has_events() {
                    self.events.append(&mut events);
                    self.stack.push(events.get_events());
                }
                loop {
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
                        self.cm.$c_eb = $em::new();
                        // Run the system
                        if let Some(s) = self.services.get(&e).and_then(|v_s| v_s.get(i)) {
                            (s)(&mut self.cm, &mut self.events);
                        }
                        // If this is the last system, remove the event
                        if i + 1 >= n {
                            self.events.pop(e);
                        }
                        // Add new events
                        if self.cm.$c_eb.has_events() {
                            self.events.append(&mut self.cm.$c_eb);
                            self.stack.push(self.cm.$c_eb.get_events());
                        }
                        continue;
                    } else {
                        self.pop();
                    }
                    break;
                }
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

            pub fn add_system(&mut self, e: E, f: Box<dyn Fn(&mut $cm, &mut $em)>) {
                if let Some(v) = self.services.get_mut(&e) {
                    v.push(f);
                } else {
                    self.services.insert(e, vec![f]);
                }
            }

            pub fn add_systems(&mut self) {
                $(
                    let f = |cm: &mut $cm, em: &mut $em, f: &dyn Fn(&$e_t, $($c_ts,)* $($g_ts,)*)| {
                        if let Some(e) = em.get_event() {
                            function_body!(cm, f, e,
                                c($($c_vs, $c_ts),*),
                                g($($g_vs, $g_ts),*)
                            );
                        }
                    };
                    $(
                        self.add_system(E::$e_v, Box::new(move |cm: &mut $cm, e: &mut $em| f(cm, e, &$f)));
                    )*
                )*
            }
        }
    };
}
