use crate::{sdl2, utils::rect::Dimensions};

use super::{
    renderer::{RendererAccess, RendererTrait},
    shapes::{Rectangle, ShapeTrait},
    texture::{Texture, TextureAccess, TextureTrait},
};

// Trait for anything that wants to draw on a texture builder
pub trait Drawable {
    fn draw(&self, r: &impl RendererTrait);
}

pub struct TextureBuilder {
    target: Option<TextureAccess>,
    r: RendererAccess,
}

impl TextureBuilder {
    pub fn new(
        r: RendererAccess,
        w: i32,
        h: i32,
        bkgrnd: sdl2::SDL_Color,
    ) -> (Self, Option<Texture>) {
        let tex = r.create_texture(w, h);
        let s = Self {
            target: tex.as_ref().map_or(None, |t| Some(t.access())),
            r,
        };
        s.fill(bkgrnd);
        (s, tex)
    }

    pub fn copy_texture(r: RendererAccess, src: TextureAccess) -> (Self, Option<Texture>) {
        let dim = src.get_size();
        let tex = r.create_texture(dim.w, dim.h);
        (
            Self {
                target: tex.as_ref().map_or(None, |t| Some(t.access())),
                r,
            },
            tex,
        )
    }

    pub fn open_texture(r: RendererAccess, src: TextureAccess) -> Self {
        Self {
            target: Some(src),
            r,
        }
    }

    pub fn fill(&self, bkgrnd: sdl2::SDL_Color) {
        self.draw(Rectangle::new().set_color(bkgrnd))
    }

    // Get texture
    pub fn get(&self) -> Option<TextureAccess> {
        self.target
    }

    pub fn get_dim(&self) -> Dimensions<i32> {
        match self.get() {
            Some(t) => t.get_size(),
            None => todo!(),
        }
    }

    // Draw textures/text
    pub fn draw(&self, drawable: impl Drawable) {
        self.r.set_target(self.target);
        drawable.draw(&self.r);
        self.r.clear_target();
    }

    // Brighten texture
    pub fn brighten(&self, strength: u8) {
        self.draw(
            Rectangle::new()
                .set_color(sdl2::SDL_Color {
                    r: strength,
                    g: strength,
                    b: strength,
                    a: 255,
                })
                .set_blendmode(sdl2::SDL_BlendMode::SDL_BLENDMODE_ADD),
        )
    }
}
