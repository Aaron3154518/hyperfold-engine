#![feature(const_type_id)]
#![feature(map_many_mut)]
#![feature(hash_raw_entry)]

pub mod ecs;
pub mod framework;
pub mod includes;
mod sdl2_bindings;
mod sdl2_image_bindings;
#[cfg(feature = "test")]
pub mod test;
pub mod utils;

use hyperfold_macros::component_manager;
pub use sdl2_bindings::sdl2_ as sdl2;
pub use sdl2_image_bindings::sdl2_image_ as sdl2_image;

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

pub fn run() {
    let mut f = init_sdl();

    f.run();

    quit_sdl(f);
}
