use std::ptr::null;

use crate::utils::rect::{Dimensions, Rect};

use super::{
    asset_manager::Asset,
    drawable::Drawable,
    render_system::{RenderSystem, RenderSystemTrait},
    texture::{Texture, TextureTrait},
};
#[macros::component]
pub struct RenderData {
    asset: Asset,
    pos: Rect,
}

impl RenderData {
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

impl From<Texture> for RenderData {
    fn from(tex: Texture) -> Self {
        let dim = tex.get_size();
        Self {
            asset: Asset::Texture(tex),
            pos: Rect {
                x: 0.0,
                y: 0.0,
                w: dim.w as f32,
                h: dim.h as f32,
            },
        }
    }
}

impl Drawable for RenderData {
    fn draw(&self, rs: &mut RenderSystem) {
        rs.draw_asset(&self.asset, null(), &self.pos.to_sdl_rect())
    }
}
