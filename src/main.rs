#![feature(specialization)]
#![feature(const_type_id)]
#![feature(map_many_mut)]
#![feature(hash_raw_entry)]

mod sdl2_bindings;
use sdl2_bindings::sdl2_ as sdl2;

mod sdl2_image_bindings;
use sdl2_image_bindings::sdl2_image_ as sdl2_image;

mod asset_manager;

mod includes;

mod utils;

use utils::rect::{Dimensions, Rect};

mod ecs;

mod framework;

use ecs_lib::{component, component_manager, global};

mod test;

const FPS: u32 = 60;
const FRAME_TIME: u32 = 1000 / FPS;

#[component]
pub struct MainComponent {}

#[global(Dummy)]
struct EFoo;

#[global(Dummy)]
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

    let screen = Dimensions { w, h };
    let camera = Rect {
        x: 0.0,
        y: 0.0,
        w: w as f32,
        h: h as f32,
    };

    let mut f = SFoo::new();

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
