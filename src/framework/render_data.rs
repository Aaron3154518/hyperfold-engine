use std::ptr::null;

use crate::utils::rect::Rect;

use super::{
    texture::{TextureAccess, TextureTrait},
    texture_builder::Drawable,
};

pub struct RenderData {
    tex: TextureAccess,
    pos: Rect,
}

impl RenderData {
    pub fn new(tex: TextureAccess) -> Self {
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

impl Drawable for RenderData {
    fn draw(&self, r: &impl super::renderer::RendererTrait) {
        r.draw(&self.tex, null(), &self.pos.to_sdl_rect())
    }
}
