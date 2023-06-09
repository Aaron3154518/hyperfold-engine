#![feature(hash_drain_filter)]
#![feature(pattern)]
#![feature(map_try_insert)]
#![feature(lazy_cell)]
#![feature(trivial_bounds)]

mod sdl2_bindings;
pub use sdl2_bindings::sdl2;
mod sdl2_image_bindings;
pub use sdl2_image_bindings::sdl2_image;
mod sdl2_ttf_bindings;
pub use sdl2_ttf_bindings::sdl2_ttf;

pub use macros::{component, event, game_crate, global, system};
pub mod ecs;
pub mod framework;
pub mod intersect;
pub mod utils;

game_crate!();

pub mod test {
    use crate::ecs::{
        entities::{Entity, NewEntity},
        events::Event,
    };

    // Labels tests
    pub mod A {
        pub mod B {
            pub struct C;
        }
    }

    // crate::components!(AA);
    // crate::components!(labels(), Z);
    // crate::components!(labels((!u8)), Y);
    // crate::components!(labels(u8), X);
    // crate::components!(labels((u8)), W);
    // crate::components!(labels(u8 && i8), V);
    // crate::components!(labels((u8 || i8) && !!!u8), U);
    // crate::components!(labels((!(u8 || !i8) && u8)), Q);
    // crate::components!(labels(!A::B::C), S);
    // crate::components!(labels(String || !A::B::C), R);

    // crate::components!(
    //     labels((TFoo || !A::B::C) && !A::B::C),
    //     QuxComponents,
    //     t: &'a TFoo,
    //     greet: &'a String,
    //     happy: &'a mut bool
    // );

    pub struct QuxComponents<'a> {
        pub eid: &'a Entity,
        pub t: &'a TFoo,
        pub greet: &'a String,
        pub happy: &'a mut bool,
    }
    impl<'a> QuxComponents<'a> {
        pub fn new(eid: &'a Entity, t: &'a TFoo, greet: &'a String, happy: &'a mut bool) -> Self {
            Self {
                eid,
                t,
                greet,
                happy,
            }
        }
    }

    trait T {
        fn to_string(&self) -> String {
            return "Poopy".to_string();
        }
    }

    pub struct TFoo;
    impl T for TFoo {}

    fn qux(
        ev: Event<u32>,
        components: Vec<QuxComponents>,
        (cnt, timer, tr): (&mut u8, &u8, &dyn T),
    ) {
        for c in components {
            *cnt += 1;
            println!(
                "{ev},({},{},{}),({cnt},{timer},{})",
                c.t.to_string(),
                c.greet,
                c.happy,
                tr.to_string(),
            )
        }
    }

    pub fn test() {
        let event = 2;
        let eid = Entity::new();
        let greet = "Hello".to_string();
        let mut cnt = 0;
        let timer = 4;

        let eids = [0; 5].map(|_| Entity::new());
        let mut happies = [0, 1, 2, 3, 4].map(|i| i % 2 == 0);
        let ints = [-1, -2, -3, -4, -5];
        let t = TFoo;

        // let k = Components::new(&eid, (&greet, &mut happy));
        qux(
            &event,
            eids.iter()
                .zip(happies.iter_mut())
                .map(|(eid, happy)| QuxComponents::new(eid, &t, &greet, happy))
                .collect(),
            (&mut cnt, &timer, &t),
        );
    }
}

pub fn init_sdl() {
    // Initialize SDL2
    if unsafe { sdl2::SDL_Init(sdl2::SDL_INIT_EVERYTHING) } == 0 {
        println!("SDL Initialized");
    } else {
        panic!("SDL failed to initialize");
    }

    // Initialize SDL2_image
    let img_init_flags = sdl2_image::IMG_InitFlags::IMG_INIT_PNG as i32
        | sdl2_image::IMG_InitFlags::IMG_INIT_JPG as i32;
    if unsafe { sdl2_image::IMG_Init(img_init_flags) } & img_init_flags == img_init_flags {
        println!("SDL_Image Initialized");
    } else {
        panic!("SDL_Image failed to initialize");
    }

    // Initialize SDL2_ttf
    if unsafe { sdl2_ttf::TTF_Init() } == 0 {
        println!("SDL_TTF Initialized");
    } else {
        panic!("SDL_TTF failed to initialize");
    }
}

pub fn quit_sdl() {
    unsafe {
        sdl2_ttf::TTF_Quit();
        sdl2_image::IMG_Quit();
        sdl2::SDL_Quit();
    }
}

// fn main() {
//     init_sdl();

//     let mut sfoo = _engine::SFoo::new();
//     sfoo.run();
//     drop(sfoo);

//     quit_sdl();
// }
