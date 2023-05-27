use uuid::Uuid;

use crate::{
    ecs::{entities::Entity, events::core::Update},
    utils::rect::{Dimensions, Rect},
};

use super::{
    drawable::{AssetDrawable, Drawable},
    Asset, AssetManager, Renderer, Texture,
};

// RenderData
pub trait RenderDataTrait
where
    Self: Sized,
{
    fn get_render_data<'a>(&'a mut self) -> &'a mut RenderData;

    fn from_pos(mut self, pos: Rect) -> Self {
        self.set_pos(pos);
        self
    }

    fn set_pos(&mut self, pos: Rect) {
        self.get_render_data().pos = pos;
    }

    fn from_area(mut self, area: Option<Rect>) -> Self {
        self.set_area(area);
        self
    }

    fn set_area(&mut self, area: Option<Rect>) {
        self.get_render_data().area = area;
    }

    fn animate(
        mut self,
        entities: &mut dyn crate::_engine::AddComponent,
        eid: Entity,
        anim: Animation,
    ) -> Self {
        let rd = self.get_render_data();
        if rd.dim.w % anim.num_frames != 0 {
            eprintln!(
                "Spritesheet of length {} does not evenly divide into {} frames",
                rd.dim.w, anim.num_frames
            );
        }
        if rd.area.is_none() {
            rd.area = Some(Rect {
                x: 0.0,
                y: 0.0,
                w: (rd.dim.w / anim.num_frames) as f32,
                h: rd.dim.h as f32,
            });
        }
        entities.add_component(eid, anim);
        self
    }
}

#[derive(Debug)]
pub struct RenderData {
    pos: Rect,
    area: Option<Rect>,
    dim: Dimensions<u32>,
}

impl RenderData {
    pub fn new(dim: Dimensions<u32>) -> Self {
        Self {
            pos: Rect::new(),
            area: None,
            dim,
        }
    }
}

// RenderTexture
// Keep this separate from RenderAsset so we can call draw() without needing an AssetManager
pub struct RenderTexture {
    tex: Texture,
    data: RenderData,
}

impl RenderTexture {
    pub fn new(tex: Texture) -> Self {
        let dim = tex.get_size();
        Self {
            tex,
            data: RenderData::new(dim),
        }
    }
}

impl RenderDataTrait for RenderTexture {
    fn get_render_data<'a>(&'a mut self) -> &'a mut RenderData {
        &mut self.data
    }
}

impl Drawable for RenderTexture {
    fn draw(&self, r: &Renderer) {
        r.draw_texture(&self.tex, self.data.area, Some(self.data.pos))
    }
}

// RenderAsset
pub struct RenderAsset {
    asset: Asset,
    data: RenderData,
}

impl RenderAsset {
    pub fn new(asset: Asset, r: &Renderer, am: &mut AssetManager) -> Self {
        let dim = am
            .load_asset(r, &asset)
            .map_or(Dimensions { w: 0, h: 0 }, |t| t.get_size());
        Self {
            asset,
            data: RenderData::new(dim),
        }
    }
}

impl RenderDataTrait for RenderAsset {
    fn get_render_data<'a>(&'a mut self) -> &'a mut RenderData {
        &mut self.data
    }
}

impl AssetDrawable for RenderAsset {
    fn draw(&self, r: &Renderer, am: &mut AssetManager) {
        if let Some(tex) = am.load_asset(r, &self.asset) {
            r.draw_texture(tex, self.data.area, Some(self.data.pos));
        }
    }
}

// RenderComponent
#[macros::component]
enum RenderComponent {
    Asset(RenderAsset),
    Texture(RenderTexture),
}

impl RenderComponent {
    pub fn from_file(file: String, r: &Renderer, am: &mut AssetManager) -> Self {
        Self::Asset(RenderAsset::new(Asset::File(file), r, am))
    }

    pub fn from_id(id: Uuid, r: &Renderer, am: &mut AssetManager) -> Self {
        Self::Asset(RenderAsset::new(Asset::Id(id), r, am))
    }

    pub fn from_texture(tex: Texture) -> Self {
        Self::Texture(RenderTexture::new(tex))
    }
}

impl RenderDataTrait for RenderComponent {
    fn get_render_data<'a>(&'a mut self) -> &'a mut RenderData {
        match self {
            RenderComponent::Asset(a) => a.get_render_data(),
            RenderComponent::Texture(t) => t.get_render_data(),
        }
    }
}

impl AssetDrawable for RenderComponent {
    fn draw(&self, r: &Renderer, am: &mut AssetManager) {
        match self {
            RenderComponent::Asset(a) => a.draw(r, am),
            RenderComponent::Texture(t) => t.draw(r),
        }
    }
}

// Animation
#[macros::component]
struct Animation {
    num_frames: u32,
    frame: u32,
    mspf: u32,
    timer: u32,
}

impl Animation {
    pub fn new(num_frames: u32, mspf: u32) -> Self {
        Self {
            num_frames,
            frame: 0,
            mspf,
            timer: 0,
        }
    }
}

#[macros::system]
pub fn update_animations(update: &Update, anim: &mut Animation, rc: &mut RenderComponent) {
    anim.timer += update.0;
    if anim.timer >= anim.mspf {
        anim.frame = (anim.frame + anim.timer / anim.mspf) % anim.num_frames;
        anim.timer %= anim.mspf;

        let rd = rc.get_render_data();
        let frame_size = rd.dim.w / anim.num_frames;
        match &mut rd.area {
            Some(a) => {
                a.x = (a.x + frame_size as f32) % rd.dim.w as f32;
            }
            None => {
                rd.area = Some(Rect {
                    x: (rd.dim.w * anim.frame) as f32,
                    y: 0.0,
                    w: frame_size as f32,
                    h: rd.dim.h as f32,
                })
            }
        }
    }
}
