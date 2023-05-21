use std::ptr::null;

use crate::utils::rect::{Dimensions, Rect};

use super::{
    drawable::{AssetDrawable, Drawable},
    render_system::{Asset, Texture, TextureRendererTrait},
    render_system::{AssetManager, RenderSystemTrait, Renderer},
};

// RenderData
pub trait RenderDataTrait
where
    Self: Sized,
{
    fn render_data(&mut self) -> &mut RenderData;

    fn set_pos(mut self, pos: Rect) -> Self {
        self.render_data().pos = pos;
        self
    }
}

pub struct RenderData {
    pos: Rect,
}

impl RenderData {
    pub fn new() -> Self {
        Self { pos: Rect::new() }
    }
}

// RenderTexture
pub struct RenderTexture {
    tex: Texture,
    data: RenderData,
}

impl RenderTexture {
    pub fn new(tex: Texture) -> Self {
        let dim = tex.get_size();
        Self {
            tex,
            data: RenderData::new(),
        }
        .set_pos(Rect {
            x: 0.0,
            y: 0.0,
            w: dim.w as f32,
            h: dim.h as f32,
        })
    }
}

impl RenderDataTrait for RenderTexture {
    fn render_data(&mut self) -> &mut RenderData {
        &mut self.data
    }
}

impl Drawable for RenderTexture {
    fn draw(&self, r: &Renderer) {
        (r, &self.tex).draw_texture(null(), &self.data.pos.to_sdl_rect())
    }
}

// RenderAsset
pub struct RenderAsset {
    asset: Asset,
    data: RenderData,
}

impl RenderAsset {
    pub fn new(asset: Asset, r: &Renderer, am: &mut AssetManager) -> Self {
        let dim = (r, am)
            .load_asset(&asset)
            .map_or(Dimensions { w: 0, h: 0 }, |t| t.get_size());
        Self {
            asset,
            data: RenderData::new(),
        }
        .set_pos(Rect {
            x: 0.0,
            y: 0.0,
            w: dim.w as f32,
            h: dim.h as f32,
        })
    }
}

impl RenderDataTrait for RenderAsset {
    fn render_data(&mut self) -> &mut RenderData {
        &mut self.data
    }
}

impl AssetDrawable for RenderAsset {
    fn draw(&self, r: &Renderer, am: &mut AssetManager) {
        (r, am).draw_asset(&self.asset, null(), &self.data.pos.to_sdl_rect())
    }
}
