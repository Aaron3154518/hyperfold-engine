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

pub use ecs::ManagerTrait;

game_crate!();

fn init_sdl() {
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

fn quit_sdl() {
    unsafe {
        sdl2_ttf::TTF_Quit();
        sdl2_image::IMG_Quit();
        sdl2::SDL_Quit();
    }
}

pub fn run<T>()
where
    T: ManagerTrait,
{
    init_sdl();

    let mut t = T::new();
    t.run();
    drop(t);

    quit_sdl();
}
