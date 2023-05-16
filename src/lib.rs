#![feature(hash_drain_filter)]

mod sdl2_bindings;

use ecs::{
    components::{Components, ComponentsVec, Globals},
    entities::{Entity, NewEntity},
    events::Event,
};
pub use sdl2_bindings::sdl2_ as sdl2;
mod sdl2_image_bindings;
pub use sdl2_image_bindings::sdl2_image_ as sdl2_image;

pub use macros::{component, event, game_crate, global, system};
pub mod ecs;
pub mod framework;
pub mod intersect;
pub mod utils;

game_crate!();

// struct Query<'a, E, C, G> {
//     pub event: &'a E,
//     pub eid: &'a u32,
//     pub components: C,
//     pub globals: G,
//     // label: std::marker::PhantomData<L>,
// }

// struct FooComponents<'a> {
//     greet: &'a String,
//     happy: &'a mut bool,
// }

// struct FooGlobals<'a> {
//     cnt: &'a u8,
//     timer: &'a u8,
// }

// fn foo(q: Query<u32, FooComponents, FooGlobals>) {
//     let Query {
//         event,
//         eid,
//         components: FooComponents { greet, happy },
//         globals: FooGlobals { cnt, timer },
//     } = q;
//     *happy = !*happy;
//     println!("{event},{eid},({greet},{happy}),({cnt},{timer})")
// }

// macros::query2!(BarComponents, greet: String, happy: mut bool);
// macros::query2!(BarGlobals, cnt: u8, timer: u8);
// fn bar(q: Query<u32, BarComponents, BarGlobals>) {
//     let Query {
//         event,
//         eid,
//         components: BarComponents { greet, happy },
//         globals: BarGlobals { cnt, timer },
//     } = q;
//     *happy = !*happy;
//     println!("{event},{eid},({greet},{happy}),({cnt},{timer})")
// }

// macros::query!(
//     baz,
//     Query,
//     u32,
//     (greet: String, happy: mut bool),
//     (cnt: u8, timer: u8),
//     {
//         *happy = !*happy;
//         println!("{event},{eid},({greet},{happy}),({cnt},{timer})")
//     }
// );

trait T {
    fn to_string(&self) -> String {
        return "Poopy".to_string();
    }
}

struct TFoo;
impl T for TFoo {}

fn qux(
    ev: Event<u32>,
    Components {
        eid,
        data: (greet, happy),
        ..
    }: Components<(&String, &mut bool), ()>,
    c_vec: ComponentsVec<(&i32,), ()>,
    (cnt, timer, tr): Globals<(&u8, &u8, &dyn T)>,
) {
    *happy = true;
    println!(
        "{ev},{eid},({greet},{happy}),({cnt},{timer},{})\n{}",
        tr.to_string(),
        c_vec
            .iter()
            .map(|c| format!("{}{}=0", c.eid, c.data.0))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub fn test() {
    let event = 2;
    let eid = Entity::new();
    let greet = "Hello".to_string();
    let mut happy = false;
    let cnt = 0;
    let timer = 4;

    // let q = Query {
    //     event: &event,
    //     eid: &eid,
    //     components: (&greet, &mut happy),
    //     globals: (&cnt, &timer),
    //     // label: std::marker::PhantomData::<u8>,
    // };
    // baz(q);

    // let q = Query {
    //     event: &event,
    //     eid: &eid,
    //     components: FooComponents {
    //         greet: &greet,
    //         happy: &mut happy,
    //     },
    //     globals: FooGlobals {
    //         cnt: &cnt,
    //         timer: &timer,
    //     },
    //     // label: std::marker::PhantomData::<u8>,
    // };
    // foo(q);

    let eids = [0; 5].map(|_| Entity::new());
    let ints = [-1, -2, -3, -4, -5];
    let t = TFoo;

    let k = Components::new(&eid, (&greet, &mut happy));
    qux(
        &event,
        k,
        eids.iter()
            .zip(ints.iter())
            .map(|(eid, is)| Components::new(eid, (is,)))
            .collect(),
        (&cnt, &timer, &t),
    );

    // let q = Query {
    //     event: &event,
    //     eid: &eid,
    //     components: BarComponents {
    //         greet: &greet,
    //         happy: &mut happy,
    //     },
    //     globals: BarGlobals {
    //         cnt: &cnt,
    //         timer: &timer,
    //     },
    //     // label: std::marker::PhantomData::<u8>,
    // };
    // bar(q);
}

pub fn init_sdl() {
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
}

pub fn quit_sdl() {
    unsafe {
        sdl2_image::IMG_Quit();
        sdl2::SDL_Quit();
    }
}
