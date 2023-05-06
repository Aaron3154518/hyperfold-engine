#![feature(specialization)]
#![feature(const_type_id)]
#![feature(map_many_mut)]
#![feature(hash_raw_entry)]

mod sdl2_bindings;
use hyperfold_macros::component_manager;
use sdl2_bindings::sdl2_ as sdl2;

mod sdl2_image_bindings;
use sdl2_image_bindings::sdl2_image_ as sdl2_image;

mod includes;

mod utils;

use utils::rect::{Dimensions, Rect};

mod ecs;

mod framework;

mod test;

#[ecs::global(Dummy)]
struct EFoo;

#[ecs::global(Dummy)]
struct CFoo;

component_manager!(SFoo, CFoo, GFoo, EFoo);

fn init_sdl() -> SFoo {
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

    SFoo::new()
}

fn quit_sdl(f: SFoo) {
    drop(f);

    unsafe {
        sdl2_image::IMG_Quit();
        sdl2::SDL_Quit();
    }
}

fn main() {
    let mut f = init_sdl();

    f.run();

    quit_sdl(f);
}
