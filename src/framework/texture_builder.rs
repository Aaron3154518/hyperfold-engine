use crate::{sdl2, utils::rect::Dimensions};

use super::{
    renderer::{RendererAccess, RendererTrait},
    texture::{Texture, TextureAccess, TextureTrait},
};

// Trait for anything that wants to draw on a texture builder
pub trait Drawable {
    fn draw(&self, tex: &mut TextureBuilder);
}

pub enum Target {
    Owned(Texture),
    Shared(TextureAccess),
    None,
}

pub struct TextureBuilder {
    target: Target,
    r: RendererAccess,
}

impl TextureBuilder {
    pub fn new(r: RendererAccess, w: i32, h: i32, bkgrnd: sdl2::SDL_Color) -> Self {
        Self {
            target: r
                .create_texture(w, h)
                .map_or(Target::None, |t| Target::Owned(t)),
            r,
        }
    }

    pub fn from_texture(r: RendererAccess, src: TextureAccess, copy: bool) -> Self {
        Self {
            target: if copy {
                let dim = src.get_size();
                r.create_texture(dim.w, dim.h).map_or(Target::None, |tex| {
                    // TODO: Render data
                    // RenderData rd(src);
                    // rd.mRect = Rect(0, 0, dim.w, dim.h);
                    // draw(rd);
                    Target::Owned(tex)
                })
            } else {
                Target::Shared(src)
            },
            r,
        }
    }

    pub fn clear(&mut self) {
        // TODO: shapes
    }

    // Get texture
    pub fn get(&self) -> Option<TextureAccess> {
        match &self.target {
            Target::Owned(t) => Some(t.access()),
            Target::Shared(t) => Some(*t),
            Target::None => None,
        }
    }

    pub fn get_dim(&self) -> Dimensions<i32> {
        match self.get() {
            Some(t) => t.get_size(),
            None => todo!(),
        }
    }

    // Draw textures/text
    pub fn draw(&mut self, drawable: impl Drawable) {
        self.r.set_target(self.get());
        drawable.draw(self);
        self.r.clear_target();
    }

    // Brighten texture
    pub fn brighten(&mut self, strength: u8) {
        //     Shapes::Rectangle r;
        // r.setColor(SDL_Color{strength, strength, strength, 255});
        // r.setBlendMode(SDL_BLENDMODE_ADD);
        // draw(r);
    }
}
