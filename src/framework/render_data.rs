use std::ptr::null;

use crate::utils::rect::Rect;

use super::{
    drawable::Drawable,
    texture::{Texture, TextureTrait},
};

#[macros::component]
pub struct RenderData<'a> {
    tex: &'a Texture,
    pos: Rect,
}

impl<'a> RenderData<'a> {
    pub fn from(tex: &'a Texture) -> Self {
        let dim = tex.get_size();
        Self {
            tex,
            pos: Rect {
                x: 0.0,
                y: 0.0,
                w: dim.w as f32,
                h: dim.h as f32,
            },
        }
    }

    pub fn set_pos(mut self, pos: Rect) -> Self {
        self.pos = pos;
        self
    }
}

impl<'a> Drawable for RenderData<'a> {
    fn draw(&self, r: &impl super::renderer::RendererTrait) {
        r.draw(self.tex, null(), &self.pos.to_sdl_rect())
    }
}
