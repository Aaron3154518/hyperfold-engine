use std::ptr::null;

use crate::utils::rect::{Dimensions, Rect};

use super::{
    drawable::Drawable,
    render_system::{Asset, RenderSystem, RenderSystemTrait},
    texture::{Texture, TextureTrait},
};
use crate::framework::renderer::RendererTrait;

pub struct RenderTexture {
    tex: Texture,
    pos: Rect,
}

impl RenderTexture {
    pub fn new(tex: Texture) -> Self {
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

impl Drawable for RenderTexture {
    fn draw(&self, rs: &mut RenderSystem) {
        rs.draw_texture(&self.tex, null(), &self.pos.to_sdl_rect())
    }
}

#[macros::component]
pub struct RenderAsset {
    asset: Asset,
    pos: Rect,
}

impl RenderAsset {
    pub fn new(asset: Asset, rs: &mut RenderSystem) -> Self {
        let dim = rs
            .load_asset(&asset)
            .map_or(Dimensions { w: 0, h: 0 }, |t| t.get_size());
        Self {
            asset,
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

impl Drawable for RenderAsset {
    fn draw(&self, rs: &mut RenderSystem) {
        rs.draw_asset(&self.asset, null(), &self.pos.to_sdl_rect())
    }
}
