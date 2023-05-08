#[macro_export]
macro_rules! sum_traits {
    ($name: ident, $body: tt) => {
        pub trait $name $body
    };

    ($name: ident, ($tr_0: path $(,$tr: path)*), $body: tt) => {
        pub trait $name : $tr_0 $(+$tr)* $body
    };
}

#[macro_export]
macro_rules! events {
    ($em: ident, $em_tr: ident, ($($deps: ident),*), $n: literal, $(($s: path, $e: ident, $v: ident)),*) => {
        #[derive(Hash, Clone, Copy, Eq, PartialEq, Debug)]
        pub enum E {
            $($e),*
        }

        pub const E_LEN: usize = $n;

        #[derive(Debug)]
        pub struct $em {
            $($v: Vec<$s>),*,
            events: std::collections::VecDeque<(E, usize)>
        }

        impl $em {
            pub fn new() -> Self {
                Self {
                    $($v: Vec::new()),*,
                    events: std::collections::VecDeque::new()
                }
            }

            pub fn has_events(&self) -> bool {
                !self.events.is_empty()
            }

            fn add_event(&mut self, e: E) {
                self.events.push_back((e, 0));
            }

            pub fn get_events(&mut self) -> std::collections::VecDeque<(E, usize)> {
                std::mem::replace(&mut self.events, std::collections::VecDeque::new())
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

        events_trait!($em_tr, $($s),*);

        impl $em_tr for $em {}
        $(impl $deps::$em_tr for $em {})*
    }
}

#[macro_export]
macro_rules! events_trait {
    ($em_tr: ident, $s0: path $(,$s: path)*) => {
        sum_traits!($em_tr, ($(Mut<$s>),*), {});
    };
}

#[macro_export]
macro_rules! c_manager {
    ($cm: ident, $cm_tr: ident, ($($deps: ident),*), $($c_v: ident, $c_t: ty),*) => {
        pub struct $cm {
            eids: std::collections::HashSet<Entity>,
            $($c_v: std::collections::HashMap<Entity, $c_t>,)*
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

            pub fn remove(&mut self, tr: &mut EntityTrash) {
                for eid in tr.0.drain(..) {
                    self.eids.remove(&eid);
                    $(self.$c_v.remove(&eid);)*
                }
            }
        }

        $(
            impl ComponentManager<Entity, $c_t> for $cm {
                fn add_component(&mut self, e: Entity, t: $c_t) {
                    self.eids.insert(e);
                    self.$c_v.insert(e, t);
                }
            }
        )*

        c_manager_trait!($cm_tr, $($c_t),*);

        impl $cm_tr for $cm {}
        $(
            impl $deps::$cm_tr for $cm {}
        )*
    };
}

#[macro_export]
macro_rules! c_manager_trait {
    ($cm_tr: ident, $($c_t: ty),*) => {
        sum_traits!($cm_tr, ($(ComponentManager<Entity, $c_t>),*), {});
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

#[macro_export]
macro_rules! systems {
    ($sm: ident, $cm: ident, $gm: ident, $em: ident,
        (
            $event_manager: ident, $event: ident, $render_system: ident,
            $component_manager: ident, $entity_trash: ident,
            $screen: ident, $camera: ident
        ),
        ($($i_fs: tt),*), ($($e_v: ident, $fs: tt),*)
    ) => {
        pub struct $sm {
            pub gm: $gm,
            pub cm: $cm,
            // Stores events in the order that they should be handled
            events: $em,
            stack: Vec<std::collections::VecDeque<(E, usize)>>,
            services: [Vec<Box<dyn Fn(&mut $cm, &mut $gm, &mut $em)>>; E_LEN]
        }

        impl $sm {
            pub fn new() -> Self {
                let mut s = Self {
                    gm: $gm::new(),
                    cm: $cm::new(),
                    events: $em::new(),
                    stack: Vec::new(),
                    services: crate::ecs::shared::array_creator::ArrayCreator::create(|_| Vec::new())
                };
                s.init();
                s
            }

            // Init
            fn init(&mut self) {
                $($i_fs(&mut self.cm, &mut self.gm, &mut self.events);)*
                self.post_tick();
                self.add_systems();
            }

            fn add_system(&mut self, e: E, f: Box<dyn Fn(&mut $cm, &mut $gm, &mut $em)>) {
                self.services[e as usize].push(f);
            }

            fn add_systems(&mut self) {
                $(
                    let (f) = $fs;
                    self.add_system(E::$e_v, Box::new(f));
                )*
            }

            // Tick
            pub fn run(&mut self) {
                static FPS: u32 = 60;
                static FRAME_TIME: u32 = 1000 / FPS;

                let mut t = unsafe { crate::sdl2::SDL_GetTicks() };
                let mut dt;
                let mut tsum: u64 = 0;
                let mut tcnt: u64 = 0;
                while !self.gm.$event.quit {
                    dt = unsafe { crate::sdl2::SDL_GetTicks() } - t;
                    t += dt;

                    self.tick(dt);

                    dt = unsafe { crate::sdl2::SDL_GetTicks() } - t;
                    tsum += dt as u64;
                    tcnt += 1;
                    if dt < FRAME_TIME {
                        unsafe { crate::sdl2::SDL_Delay(FRAME_TIME - dt) };
                    }
                }

                println!("Average Frame Time: {}ms", tsum as f64 / tcnt as f64);
            }

            fn tick(&mut self, ts: u32) {
                // Update events
                self.gm.$event.update(ts, &self.gm.$camera.0, &self.gm.$screen.0);
                // Clear the screen
                self.gm.$render_system.r.clear();
                // Add initial events
                self.add_events(self.init_events(ts));
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
                            // Increment the event idx and return the old values
                            let v_s = &self.services[*e as usize];
                            v_s.get(*i).map(|_| {
                                let vals = (e.clone(), i.clone(), v_s.len());
                                *i += 1;
                                vals
                            })
                        })
                    {
                        // This is the last system for this event
                        if i + 1 >= n {
                            self.pop();
                        }
                        // Add a new queue for new events
                        self.gm.$event_manager = $em::new();
                        // Run the system
                        if let Some(s) = self.services[e as usize].get(i) {
                            (s)(&mut self.cm, &mut self.gm, &mut self.events);
                        }
                        // If this is the last system, remove the event
                        if i + 1 >= n {
                            self.events.pop(e);
                        }
                        // Add new events
                        let events = std::mem::replace(&mut self.gm.$event_manager, $em::new());
                        self.add_events(events);
                    } else {
                        // We're done with this event
                        self.pop();
                    }
                }
                // Display the screen
                self.gm.$render_system.r.present();

                self.post_tick();
            }

            fn post_tick(&mut self) {
                // Remove marked entities
                self.cm.remove(&mut self.gm.$entity_trash);
                // Add new entities
                self.cm.append(&mut self.gm.$component_manager);
            }

            fn init_events(&self, ts: u32) -> $em {
                let mut events = $em::new();
                events.new_event(CoreEvent::Events);
                events.new_event(CoreEvent::Update(ts));
                events.new_event(CoreEvent::Render);
                events
            }

            fn add_events(&mut self, mut em: $em) {
                if em.has_events() {
                    self.events.append(&mut em);
                    self.stack.push(em.get_events());
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
        }
    };
}
