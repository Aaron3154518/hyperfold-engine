use crate::sdl2;

use super::{
    shapes::{Rectangle, ShapeTrait},
    AssetManager, Renderer, Texture,
};

// Trait for anything that wants to draw on a texture builder
pub trait Drawable {
    fn draw(&self, r: &Renderer);
}

pub trait AssetDrawable {
    fn draw(&self, r: &Renderer, am: &mut AssetManager);
}

impl Texture {
    pub fn new(r: &Renderer, w: i32, h: i32, bkgrnd: sdl2::SDL_Color) -> Self {
        let s = r.create_texture(w, h).expect("Failed to create texture");
        s.draw(r, Rectangle::new().set_color(bkgrnd));
        s
    }

    pub fn copy_texture(r: &Renderer, src: &Texture) -> Self {
        let dim = src.get_size();
        r.create_texture(dim.w, dim.h)
            .expect("Failed to create texture")
        // TODO: Copy src
    }

    // Draw textures/text
    pub fn draw(&self, r: &Renderer, drawable: impl Drawable) {
        r.set_target(self);
        drawable.draw(r);
        r.clear_target();
    }

    pub fn draw_asset(&self, r: &Renderer, am: &mut AssetManager, drawable: impl AssetDrawable) {
        r.set_target(self);
        drawable.draw(r, am);
        r.clear_target();
    }
}
