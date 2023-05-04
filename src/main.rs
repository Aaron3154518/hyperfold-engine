#![feature(specialization)]
#![feature(const_type_id)]
#![feature(map_many_mut)]
#![feature(hash_raw_entry)]

use std::{
    collections::{HashMap, VecDeque},
    hash::Hash,
};

mod sdl2_bindings;
use framework::render_system::Elevation;
use sdl2_bindings::sdl2_ as sdl2;

mod sdl2_image_bindings;
use sdl2_image_bindings::sdl2_image_ as sdl2_image;

use ecs_macros::events;

mod asset_manager;

mod utils;

use utils::rect::{Dimensions, PointF, Rect};

mod ecs;

mod framework;

use ecs_lib::{component, component_manager};

use crate::test::Player;

mod test;

const FPS: u32 = 60;
const FRAME_TIME: u32 = 1000 / FPS;

#[component]
pub struct MainComponent {}

#[component(Global, Dummy)]
struct EFoo;

#[component(Global, Dummy)]
struct CFoo;

component_manager!(SFoo, CFoo, GFoo, EFoo);

fn main() {
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

    let screen = Dimensions { w, h };
    let camera = Rect {
        x: 0.0,
        y: 0.0,
        w: w as f32,
        h: h as f32,
    };

    let mut f = SFoo::new();

    let e1 = ecs::entity::new();
    f.cm.add_component(
        e1,
        ecs::component::Component {
            name: "Aaron",
            loc: "Boise, Idaho",
        },
    );
    f.cm.add_component(
        e1,
        ecs::component::MyComponent {
            msg: "You should stop coding".to_string(),
        },
    );
    f.cm.add_component(e1, ecs::test::tmp::Component { i: 666 });
    f.cm.add_component(e1, MainComponent {});
    f.cm.add_component(e1, 1 as Elevation);
    f.cm.add_component(
        e1,
        framework::physics::Position {
            x: (w - img_w) as f32 / 2.0,
            y: (h - img_w) as f32 / 2.0,
            w: img_w as f32,
            h: img_w as f32,
        },
    );
    f.cm.add_component(
        e1,
        framework::physics::PhysicsData {
            v: PointF::new(),
            a: PointF::new(),
            boundary: camera.clone(),
        },
    );
    let tex = f.get_rs().get_image("res/bra_vector.png");
    f.cm.add_component(e1, tex);
    f.cm.add_component(e1, 1 as test::FBallTimer);
    f.cm.add_component(e1, Player);

    let e2 = ecs::entity::new();
    f.cm.add_component(
        e2,
        ecs::component::Component {
            name: "Ur Mom",
            loc: "Stoopidville",
        },
    );
    f.cm.add_component(
        e2,
        ecs::component::MyComponent {
            msg: "Lmao git gud".to_string(),
        },
    );
    f.cm.add_component(e2, ecs::test::tmp::Component { i: 69 });

    let mut t = unsafe { sdl2::SDL_GetTicks() };
    let mut dt;
    let mut tsum: u64 = 0;
    let mut tcnt: u64 = 0;
    while !f.quit() {
        dt = unsafe { sdl2::SDL_GetTicks() } - t;
        t += dt;

        f.tick(dt, &camera, &screen);

        dt = unsafe { sdl2::SDL_GetTicks() } - t;
        tsum += dt as u64;
        tcnt += 1;
        if dt < FRAME_TIME {
            unsafe { sdl2::SDL_Delay(FRAME_TIME - dt) };
        }
    }

    println!("Average Frame Time: {}ms", tsum as f64 / tcnt as f64);

    // Destroy RenderSystem
    // drop(rs);
    drop(f);

    // Destroy the window and quit SDL2
    unsafe {
        sdl2_image::IMG_Quit();
        sdl2::SDL_Quit();
    }
}
