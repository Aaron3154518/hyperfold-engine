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

pub use sdl2_bindings::sdl2_ as sdl2;
pub use sdl2_image_bindings::sdl2_image_ as sdl2_image;

ecs::component_manager!();

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
