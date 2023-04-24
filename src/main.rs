#![feature(specialization)]
#![feature(const_type_id)]

mod sdl2_bindings;
use std::{
    any::TypeId,
    collections::{HashMap, VecDeque},
    hash::Hash,
};

use sdl2_bindings::sdl2_::{self as sdl2};
mod sdl2_image_bindings;
use sdl2_image_bindings::sdl2_image_ as sdl2_image;

use asset_manager::*;

mod asset_manager;

mod utils;

use utils::{
    event::{Event, Mouse},
    pointers::Window,
    rect::{Align, Dimensions, Rect},
};

// mod ecs;

const FPS: u32 = 60;
const FRAME_TIME: u32 = 1000 / FPS;

// use ecs_lib::{component, component_manager};

// #[component]
// pub struct MainComponent {}

// component_manager!(SFoo, Foo);

//
//
//

mod T {
    pub struct A;
    pub struct B(pub i32);
    pub struct C {
        pub name: String,
    }
}
mod U {
    pub struct A;
}

macro_rules! event_manager {
    ($(($s: path, $e: ident, $v: ident)),*) => {
        #[derive(Hash, Clone, Copy, Eq, PartialEq, Debug)]
        pub enum E {
            $($e),*
        }

        pub struct EM {
            $($v: Vec<$s>,)*
            events: VecDeque<(E, usize)>
        }

        impl EM {
            pub fn new() -> Self {
                Self {
                    $($v: Vec::new(),)*
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

        pub trait Mut<T> {
            fn new_event(&mut self, t: T);

            fn get_event<'a>(&'a self) -> Option<&'a T>;
        }

        $(
            impl Mut<$s> for EM {
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

event_manager!(
    (T::A, A0, e0_0),
    (T::B, B0, e0_1),
    (T::C, C0, e0_2),
    (U::A, A1, e1_0)
);

struct CM {
    events: EM,
}

impl CM {
    pub fn new() -> Self {
        Self { events: EM::new() }
    }
}

struct SM {
    cm: CM,
    stack: Vec<VecDeque<(E, usize)>>,
    services: HashMap<E, Vec<Box<dyn Fn(&mut CM, &mut EM)>>>,
    // Stores events in the order that they should be handled
    events: EM,
}

impl SM {
    pub fn new() -> Self {
        Self {
            cm: CM::new(),
            stack: Vec::new(),
            services: HashMap::new(),
            events: EM::new(),
        }
    }

    fn init(&mut self) {
        let mut q = VecDeque::new();
        q.push_front((E::B0, 0));
        self.stack.push(q);
        self.events.e0_1.push(T::B(69));
    }

    pub fn tick(&mut self) {
        self.init();
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
                self.cm.events = EM::new();
                // Run the system
                if let Some(s) = self.services.get(&e).and_then(|v_s| v_s.get(i)) {
                    (s)(&mut self.cm, &mut self.events);
                }
                // If this is the last system, remove the event
                if i + 1 >= n {
                    self.events.pop(e);
                }
                // Add new events
                if self.cm.events.has_events() {
                    self.events.append(&mut self.cm.events);
                    self.stack.push(self.cm.events.get_events());
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

    pub fn add_services(&mut self) {
        let f = Box::new(|cm: &mut CM, em: &mut EM| {
            if let Some(e) = em.get_event() {
                greet(&mut cm.events, e);
            }
        });
        self.services.insert(E::B0, vec![f]);
        let f = Box::new(|cm: &mut CM, em: &mut EM| {
            if let Some(e) = em.get_event() {
                greet2(e);
            }
        });
        self.services.insert(E::C0, vec![f.to_owned(), f]);
    }
}

fn greet(ev: &mut EM, e: &T::B) {
    println!("Hey {}", e.0);
    ev.new_event(T::C {
        name: "Hi".to_string(),
    });
    ev.new_event(T::C {
        name: "Ho".to_string(),
    });
}

fn greet2(e: &T::C) {
    println!("Hey {}", e.name);
}

fn main() {
    let mut sm = SM::new();
    sm.add_services();
    sm.tick();

    // let mut f = SFoo::new();
    // let e1 = ecs::entity::Entity::new();
    // let e2 = ecs::entity::Entity::new();

    // f.component_manager.add_component(
    //     e1,
    //     ecs::component::Component {
    //         name: "Aaron",
    //         loc: "Boise, Idaho",
    //     },
    // );
    // f.component_manager.add_component(
    //     e1,
    //     ecs::component::MyComponent {
    //         msg: "You should stop coding".to_string(),
    //     },
    // );
    // f.component_manager
    //     .add_component(e1, ecs::test::tmp::Component { i: 666 });
    // f.component_manager.add_component(e1, MainComponent {});

    // f.component_manager.add_component(
    //     e2,
    //     ecs::component::Component {
    //         name: "Ur Mom",
    //         loc: "Stoopidville",
    //     },
    // );
    // f.component_manager.add_component(
    //     e2,
    //     ecs::component::MyComponent {
    //         msg: "Lmao git gud".to_string(),
    //     },
    // );
    // f.component_manager
    //     .add_component(e2, ecs::test::tmp::Component { i: 69 });
    // // f.component_manager.add_component(e2, MainComponent {});
    // f.add_systems();
    // f.tick();

    // Initialize SDL2
    if unsafe { sdl2::SDL_Init(sdl2::SDL_INIT_EVERYTHING) } == 0 {
        println!("SDL Initialized");
    } else {
        eprintln!("SDL Failed to Initialize");
    }
    let img_init_flags = sdl2_image::IMG_InitFlags::IMG_INIT_PNG as i32
        | sdl2_image::IMG_InitFlags::IMG_INIT_JPG as i32;
    if unsafe { sdl2_image::IMG_Init(img_init_flags) } & img_init_flags == img_init_flags {
        println!("SDL_Image Initialized");
    } else {
        eprintln!("SDL_Image Failed to Initialize");
    }

    let w = 960;
    let h = 720;
    let img_w = 100;

    // Create a window
    let mut rs = RenderSystem::new(Window::new().title("Game Engine").dimensions(w, h));

    // Create ECSDriver
    // let mut ecs = ECSDriver::new();
    // ecs.add_comp(Component {
    //     name: "Aaron",
    //     loc: "Boise, Idaho",
    // });
    // ecs.add_serv(&greet);
    // ecs.run();

    let screen = Dimensions { w, h };
    let camera = Rect {
        x: 0.0,
        y: 0.0,
        w: w as f32,
        h: h as f32,
    };

    let tex = rs.get_image("res/bra_vector.png");
    let mut rect = Rect {
        x: (w - img_w) as f32 / 2.0,
        y: (h - img_w) as f32 / 2.0,
        w: img_w as f32,
        h: img_w as f32,
    };

    let mut event = Event::new();
    let mut t = unsafe { sdl2::SDL_GetTicks() };
    let mut dt;
    while !event.quit {
        dt = unsafe { sdl2::SDL_GetTicks() } - t;
        t += dt;

        event.update(dt, &camera, &screen);

        match event.get_key(sdl2::SDL_KeyCode::SDLK_SPACE) {
            Some(kb) => {
                if kb.held() {
                    println!("_ {}", kb.duration)
                }
            }
            None => (),
        }

        let l = event.get_mouse(Mouse::Left);
        if l.clicked() {
            rect.set_pos(
                l.click_pos.x as f32,
                l.click_pos.y as f32,
                Align::Center,
                Align::Center,
            );
            rect.fit_within(&camera);
        }

        // Clear the screen
        rs.r.clear();

        draw!(rs, tex, std::ptr::null(), &rect.to_sdl_rect());

        // Update the screen
        rs.r.present();

        dt = unsafe { sdl2::SDL_GetTicks() } - t;
        if dt < FRAME_TIME {
            unsafe { sdl2::SDL_Delay(FRAME_TIME - dt) };
        }
    }

    // Destroy RenderSystem
    drop(rs);

    // Destroy the window and quit SDL2
    unsafe {
        sdl2_image::IMG_Quit();
        sdl2::SDL_Quit();
    }
}
