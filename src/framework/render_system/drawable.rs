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
    pub fn new(r: &Renderer, w: u32, h: u32, bkgrnd: sdl2::SDL_Color) -> Self {
        let s = r.create_texture(w, h).expect("Failed to create texture");
        s.draw(r, &Rectangle::new().set_color(bkgrnd));
        s
    }

    pub fn copy_texture(r: &Renderer, src: &Texture) -> Self {
        let dim = src.get_size();
        r.create_texture(dim.w, dim.h)
            .expect("Failed to create texture")
        // TODO: Copy src
    }

    // Draw drawables
    pub fn draw(&self, r: &Renderer, drawable: &dyn Drawable) {
        r.set_target(self);
        drawable.draw(r);
        r.clear_target();
    }

    pub fn draw_asset(&self, r: &Renderer, am: &mut AssetManager, drawable: &dyn AssetDrawable) {
        r.set_target(self);
        drawable.draw(r, am);
        r.clear_target();
    }
}

impl Renderer {
    pub fn draw(&self, drawable: &dyn Drawable) {
        self.clear_target();
        drawable.draw(self);
    }

    pub fn draw_asset(&self, am: &mut AssetManager, drawable: &dyn AssetDrawable) {
        self.clear_target();
        drawable.draw(self, am);
    }
}
