#![feature(specialization)]
#![feature(const_type_id)]

mod sdl2_bindings;
use std::{
    any::TypeId,
    collections::{HashMap, VecDeque},
    hash::Hash,
};

use ecs_macros::events;
use sdl2_bindings::sdl2_::{self as sdl2};
mod sdl2_image_bindings;
use sdl2_image_bindings::sdl2_image_ as sdl2_image;

mod asset_manager;

use asset_manager::*;

mod utils;

use utils::{
    event::{Event, Mouse},
    pointers::Window,
    rect::{Align, Dimensions, PointF, Rect},
};

mod ecs;

mod framework;

use ecs_lib::{component, component_manager};

mod test;

const FPS: u32 = 60;
const FRAME_TIME: u32 = 1000 / FPS;

#[component]
pub struct MainComponent {}

#[component(Global, Dummy)]
struct EFoo;

component_manager!(SFoo, Foo, EFoo);

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

    // Create a window
    // let mut rs = RenderSystem::new();

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

    let mut f = SFoo::new();
    let e1 = ecs::entity::Entity::new();
    let e2 = ecs::entity::Entity::new();

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
    f.cm.add_component(
        e1,
        framework::physics::Position(Rect {
            x: (w - img_w) as f32 / 2.0,
            y: (h - img_w) as f32 / 2.0,
            w: img_w as f32,
            h: img_w as f32,
        }),
    );
    f.cm.add_component(
        e1,
        framework::physics::PhysicsData {
            v: PointF::new(),
            a: PointF::new(),
            boundary: camera.clone(),
        },
    );
    let img = framework::render_system::Image(f.get_rs().get_image("res/bra_vector.png"));
    f.cm.add_component(e1, img);

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
    while !f.quit() {
        dt = unsafe { sdl2::SDL_GetTicks() } - t;
        t += dt;

        f.tick(dt, &camera, &screen);

        dt = unsafe { sdl2::SDL_GetTicks() } - t;
        if dt < FRAME_TIME {
            unsafe { sdl2::SDL_Delay(FRAME_TIME - dt) };
        }
    }

    // Destroy RenderSystem
    // drop(rs);
    drop(f);

    // Destroy the window and quit SDL2
    unsafe {
        sdl2_image::IMG_Quit();
        sdl2::SDL_Quit();
    }
}
