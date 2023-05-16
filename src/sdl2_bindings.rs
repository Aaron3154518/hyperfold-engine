#[allow(warnings)]
pub mod sdl2 {
    include!(concat!(env!("OUT_DIR"), "/sdl2_bindings.rs"));
}
