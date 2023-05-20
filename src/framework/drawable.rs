use crate::sdl2;

use super::{
    renderer::RendererTrait,
    shapes::{Rectangle, ShapeTrait},
    texture::{Texture, TextureTrait},
};

// Trait for anything that wants to draw on a texture builder
pub trait Drawable {
    fn draw(&self, r: &impl RendererTrait);
}

impl Texture {
    pub fn new(r: &impl RendererTrait, w: i32, h: i32, bkgrnd: sdl2::SDL_Color) -> Self {
        let s = r.create_texture(w, h).expect("Failed to create texture");
        s.draw(r, Rectangle::new().set_color(bkgrnd));
        s
    }

    pub fn copy_texture(r: &impl RendererTrait, src: &Texture) -> Self {
        let dim = src.get_size();
        r.create_texture(dim.w, dim.h)
            .expect("Failed to create texture")
        // TODO: Copy src
    }

    // Draw textures/text
    pub fn draw(&self, r: &impl RendererTrait, drawable: impl Drawable) {
        r.set_target(Some(self));
        drawable.draw(r);
        r.clear_target();
    }
}
