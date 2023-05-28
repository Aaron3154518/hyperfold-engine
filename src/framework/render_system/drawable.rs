use crate::sdl2;

use super::{
    shapes::{Rectangle, ShapeTrait},
    AssetManager, Renderer, Texture,
};

// Trait for anything that wants to draw on a texture builder
pub trait Drawable {
    fn draw(&mut self, r: &Renderer);
}

impl Drawable for Vec<&mut dyn Drawable> {
    fn draw(&mut self, r: &Renderer) {
        self.iter_mut().for_each(|d| d.draw(r))
    }
}

pub trait AssetDrawable {
    fn draw(&mut self, r: &Renderer, am: &mut AssetManager);
}

impl AssetDrawable for Vec<&mut dyn AssetDrawable> {
    fn draw(&mut self, r: &Renderer, am: &mut AssetManager) {
        self.iter_mut().for_each(|d| d.draw(r, am))
    }
}

impl<T> AssetDrawable for T
where
    T: Drawable,
{
    fn draw(&mut self, r: &Renderer, am: &mut AssetManager) {
        self.draw(r);
    }
}

impl Texture {
    pub fn new(r: &Renderer, w: u32, h: u32, bkgrnd: sdl2::SDL_Color) -> Self {
        let s = r.create_texture(w, h).expect("Failed to create texture");
        s.draw(r, &mut Rectangle::new().set_color(bkgrnd));
        s
    }

    pub fn copy_texture(r: &Renderer, src: &Texture) -> Self {
        let dim = src.get_size();
        r.create_texture(dim.w, dim.h)
            .expect("Failed to create texture")
        // TODO: Copy src
    }

    // Draw drawables
    pub fn draw(&self, r: &Renderer, drawable: &mut dyn Drawable) {
        r.set_target(self);
        drawable.draw(r);
        r.clear_target();
    }

    pub fn draw_asset(
        &self,
        r: &Renderer,
        am: &mut AssetManager,
        drawable: &mut dyn AssetDrawable,
    ) {
        r.set_target(self);
        drawable.draw(r, am);
        r.clear_target();
    }
}

impl Renderer {
    pub fn draw(&self, drawable: &mut dyn Drawable) {
        self.clear_target();
        drawable.draw(self);
    }

    pub fn draw_asset(&self, am: &mut AssetManager, drawable: &mut dyn AssetDrawable) {
        self.clear_target();
        drawable.draw(self, am);
    }
}
