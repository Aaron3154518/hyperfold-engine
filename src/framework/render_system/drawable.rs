use crate::{impl_as_any_for_trait, sdl2, utils::util::AsAny};

use super::{
    shapes::{Rectangle, ShapeTrait},
    AssetManager, Renderer, Texture,
};

// Trait for anything that wants to draw on a texture builder
pub trait Drawable: AsAny {
    fn draw(&mut self, r: &Renderer);
}

pub trait AssetDrawable: AsAny {
    fn draw(&mut self, r: &Renderer, am: &mut AssetManager);
}

impl_as_any_for_trait!(AssetDrawable);

impl<T> AssetDrawable for T
where
    T: Drawable,
{
    fn draw(&mut self, r: &Renderer, _am: &mut AssetManager) {
        self.draw(r);
    }
}

pub trait Canvas {
    fn set_target(&self, r: &Renderer);

    fn draw(&self, r: &Renderer, drawable: &mut dyn Drawable) {
        self.set_target(r);
        drawable.draw(r);
        r.clear_target();
    }

    fn draw_many(&self, r: &Renderer, mut drawables: Vec<&mut dyn Drawable>) {
        self.set_target(r);
        drawables.iter_mut().for_each(|d| d.draw(r));
        r.clear_target();
    }

    fn draw_asset(&self, r: &Renderer, am: &mut AssetManager, drawable: &mut dyn AssetDrawable) {
        self.set_target(r);
        drawable.draw(r, am);
        r.clear_target();
    }

    fn draw_asset_many(
        &self,
        r: &Renderer,
        am: &mut AssetManager,
        mut drawables: Vec<&mut dyn AssetDrawable>,
    ) {
        self.set_target(r);
        drawables.iter_mut().for_each(|d| d.draw(r, am));
        r.clear_target();
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
}

impl Canvas for Texture {
    fn set_target(&self, r: &Renderer) {
        r.set_target(self);
    }
}

impl Canvas for Renderer {
    fn set_target(&self, r: &Renderer) {
        r.clear_target();
    }
}
