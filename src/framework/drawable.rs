use crate::sdl2;

use super::{
    render_system::RenderSystem,
    renderer::RendererTrait,
    shapes::{Rectangle, ShapeTrait},
    texture::{Texture, TextureTrait},
};

// Trait for anything that wants to draw on a texture builder
pub trait Drawable {
    fn draw(&self, rs: &mut RenderSystem);
}

impl Texture {
    pub fn new(rs: &mut RenderSystem, w: i32, h: i32, bkgrnd: sdl2::SDL_Color) -> Self {
        let s = rs.create_texture(w, h).expect("Failed to create texture");
        s.draw(rs, Rectangle::new().set_color(bkgrnd));
        s
    }

    pub fn copy_texture(r: &impl RendererTrait, src: &Texture) -> Self {
        let dim = src.get_size();
        r.create_texture(dim.w, dim.h)
            .expect("Failed to create texture")
        // TODO: Copy src
    }

    // Draw textures/text
    pub fn draw(&self, rs: &mut RenderSystem, drawable: impl Drawable) {
        rs.set_target(Some(self));
        drawable.draw(rs);
        rs.clear_target();
    }
}
