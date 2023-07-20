use uuid::Uuid;

use crate::{
    components,
    ecs::{
        entities::Entity,
        events::core::{PreRender, Update},
    },
    framework::physics::Position,
    utils::{
        rect::{Align, Dimensions, Rect},
        util::{AsType, TryAsType},
    },
};

use super::{
    drawable::{AssetDrawable, Drawable},
    rect_to_camera_coords, Asset, AssetManager, Camera, Renderer, Screen, Texture,
};

// RenderData

#[derive(Copy, Clone, Debug)]
pub enum RectMode {
    Absolute,
    Percent,
}

#[derive(Copy, Clone, Debug)]
pub enum FitMode {
    Exact,
    FitWithin(Align, Align),
}

#[derive(Copy, Clone, Debug)]
pub struct Destination {
    pub(super) rect: Rect,
    pub(super) mode: RectMode,
    pub(super) fit: FitMode,
}

impl Destination {
    pub fn to_rect(&self, dim: Dimensions<u32>) -> Rect {
        let (w, h) = (dim.w as f32, dim.h as f32);
        let r = match self.mode {
            RectMode::Absolute => self.rect,
            RectMode::Percent => Rect {
                x: self.rect.x * w,
                y: self.rect.y * h,
                w: self.rect.w * w,
                h: self.rect.h * h,
            },
        };
        match self.fit {
            FitMode::Exact => r,
            FitMode::FitWithin(ax, ay) => r.fit_dim_within(w, h).with_rect_pos(r, ax, ay),
        }
    }
}

#[derive(Debug)]
pub struct RenderData {
    pub(super) dest: Destination,
    pub(super) dest_rect: Rect,
    pub(super) area: Option<Rect>,
    dim: Dimensions<u32>,
}

impl RenderData {
    pub fn new(dim: Dimensions<u32>) -> Self {
        let dest = Destination {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: dim.w as f32,
                h: dim.h as f32,
            },
            mode: RectMode::Absolute,
            fit: FitMode::Exact,
        };
        Self {
            dest,
            dest_rect: dest.to_rect(dim),
            area: None,
            dim,
        }
    }
}

pub trait RenderDataTrait {
    fn get_render_data<'a>(&'a self) -> &'a RenderData;

    fn get_render_data_mut<'a>(&'a mut self) -> &'a mut RenderData;

    fn set_dim(&mut self, dim: Dimensions<u32>) {
        let rd = self.get_render_data_mut();
        rd.dim = dim;
        rd.dest_rect = rd.dest.to_rect(dim);
    }

    fn set_dest(&mut self, rect: Rect, mode: RectMode, fit: FitMode) {
        let rd = self.get_render_data_mut();
        rd.dest = Destination { rect, mode, fit };
        rd.dest_rect = rd.dest.to_rect(rd.dim);
    }

    fn set_dest_rect(&mut self, rect: Rect) {
        let rd = self.get_render_data_mut();
        rd.dest.rect = rect;
        rd.dest_rect = rd.dest.to_rect(rd.dim);
    }

    fn set_dest_mode(&mut self, mode: RectMode) {
        let rd = self.get_render_data_mut();
        rd.dest.mode = mode;
        rd.dest_rect = rd.dest.to_rect(rd.dim);
    }

    fn set_dest_fit(&mut self, fit: FitMode) {
        let rd = self.get_render_data_mut();
        rd.dest.fit = fit;
        rd.dest_rect = rd.dest.to_rect(rd.dim);
    }

    fn set_area(&mut self, area: Option<Rect>) {
        self.get_render_data_mut().area = area;
    }

    fn animate(
        &mut self,
        entities: &mut dyn crate::_engine::AddComponent,
        eid: Entity,
        anim: Animation,
    ) {
        let rd = self.get_render_data_mut();
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
    }
}

pub trait RenderDataBuilderTrait
where
    Self: Sized + RenderDataTrait,
{
    fn fill_dest(self) -> Self {
        self.with_dest_rect(Rect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        })
        .with_dest_mode(RectMode::Percent)
    }

    fn with_dest(mut self, rect: Rect, mode: RectMode, fit: FitMode) -> Self {
        self.set_dest(rect, mode, fit);
        self
    }

    fn with_dest_rect(mut self, rect: Rect) -> Self {
        self.set_dest_rect(rect);
        self
    }

    fn with_dest_mode(mut self, mode: RectMode) -> Self {
        self.set_dest_mode(mode);
        self
    }

    fn with_dest_fit(mut self, fit: FitMode) -> Self {
        self.set_dest_fit(fit);
        self
    }

    fn with_area(mut self, area: Option<Rect>) -> Self {
        self.set_area(area);
        self
    }

    fn with_animation(
        mut self,
        entities: &mut dyn crate::_engine::AddComponent,
        eid: Entity,
        anim: Animation,
    ) -> Self {
        self.animate(entities, eid, anim);
        self
    }
}

impl<T> RenderDataBuilderTrait for T where T: RenderDataTrait {}

impl RenderDataTrait for RenderData {
    fn get_render_data<'a>(&'a self) -> &'a RenderData {
        self
    }

    fn get_render_data_mut<'a>(&'a mut self) -> &'a mut RenderData {
        self
    }
}

// RenderTexture
// Keep this separate from RenderAsset so we can call draw() without needing an AssetManager
pub struct RenderTexture {
    tex: Option<Texture>,
    data: RenderData,
}

impl RenderTexture {
    pub fn new(tex: Option<Texture>) -> Self {
        Self {
            tex: None,
            data: RenderData::new(Dimensions::<u32>::new()),
        }
        .with_texture(tex)
    }

    pub fn get_texture<'a>(&'a self) -> Option<&'a Texture> {
        self.tex.as_ref()
    }

    pub fn get_or_insert_texture<'a, F>(&'a mut self, f: F) -> &'a Texture
    where
        F: FnOnce() -> Texture,
    {
        let tex = self.tex.get_or_insert_with(f);
        self.data.set_dim(tex.get_size());
        tex
    }

    pub fn with_texture(mut self, tex: Option<Texture>) -> Self {
        self.set_texture(tex);
        self
    }

    pub fn set_texture(&mut self, tex: Option<Texture>) {
        self.tex = tex;
        self.data.set_dim(
            self.tex
                .as_ref()
                .map_or(Dimensions::<u32>::new(), |t| t.get_size()),
        );
    }
}

impl RenderDataTrait for RenderTexture {
    fn get_render_data<'a>(&'a self) -> &'a RenderData {
        &self.data
    }

    fn get_render_data_mut<'a>(&'a mut self) -> &'a mut RenderData {
        &mut self.data
    }
}

impl Drawable for RenderTexture {
    fn draw(&mut self, r: &Renderer) {
        if let Some(tex) = &self.tex {
            r.draw_texture(tex, self.data.area, Some(self.data.dest_rect))
        }
    }
}

// RenderAsset
pub struct RenderAsset {
    asset: Asset,
    data: RenderData,
}

impl RenderAsset {
    pub fn new(asset: Asset, r: &Renderer, am: &mut AssetManager) -> Self {
        Self {
            asset: Asset::File(String::new()),
            data: RenderData::new(Dimensions::<u32>::new()),
        }
        .with_asset(asset, r, am)
    }

    pub fn from_file(file: String, r: &Renderer, am: &mut AssetManager) -> Self {
        Self::new(Asset::File(file), r, am)
    }

    pub fn from_id(id: Uuid, r: &Renderer, am: &mut AssetManager) -> Self {
        Self::new(Asset::Id(id), r, am)
    }

    pub fn get_asset<'a>(&'a self) -> &'a Asset {
        &self.asset
    }

    pub fn with_asset(mut self, asset: Asset, r: &Renderer, am: &mut AssetManager) -> Self {
        self.set_asset(asset, r, am);
        self
    }

    pub fn set_asset(&mut self, asset: Asset, r: &Renderer, am: &mut AssetManager) {
        self.asset = asset;
        self.data.set_dim(
            am.load_asset(r, &self.asset)
                .map_or(Dimensions::<u32>::new(), |t| t.get_size()),
        );
    }
}

impl RenderDataTrait for RenderAsset {
    fn get_render_data<'a>(&'a self) -> &'a RenderData {
        &self.data
    }

    fn get_render_data_mut<'a>(&'a mut self) -> &'a mut RenderData {
        &mut self.data
    }
}

impl AssetDrawable for RenderAsset {
    fn draw(&mut self, r: &Renderer, am: &mut AssetManager) {
        if let Some(tex) = am.load_asset(r, &self.asset) {
            r.draw_texture(tex, self.data.area, Some(self.data.dest_rect));
        }
    }
}

components!(RenderPos, tex: &'a mut RenderComponent, pos: &'a Position);

#[macros::system]
fn set_render_pos(_ev: &PreRender, screen: &Screen, camera: &Camera, entities: Vec<RenderPos>) {
    for RenderPos { tex, pos, .. } in entities {
        let dest = rect_to_camera_coords(&pos.0, screen, camera);
        tex.try_mut(|rt: &mut RenderTexture| rt.get_render_data_mut().set_dest_rect(dest))
            .try_mut(tex, |ra: &mut RenderAsset| {
                ra.get_render_data_mut().set_dest_rect(dest)
            });
    }
}

// RenderComponent
pub trait RenderComponentTrait: AssetDrawable + RenderDataTrait {}

impl<T> RenderComponentTrait for T where T: AssetDrawable + RenderDataTrait {}

#[macros::component]
struct RenderComponent(pub(super) Box<dyn AssetDrawable>);

impl RenderComponent {
    pub fn new(d: impl AssetDrawable + 'static) -> Self {
        Self(Box::new(d))
    }
}

impl<T> AsType<T> for RenderComponent
where
    T: AssetDrawable + 'static,
{
    fn as_type<'a>(&'a self) -> Option<&'a T> {
        self.0.as_type()
    }

    fn as_type_mut<'a>(&'a mut self) -> Option<&'a mut T> {
        self.0.as_type_mut()
    }
}

impl AssetDrawable for RenderComponent {
    fn draw(&mut self, r: &Renderer, am: &mut AssetManager) {
        self.0.draw(r, am);
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

components!(
    Animations,
    anim: &'a mut Animation,
    tex: &'a mut RenderComponent
);

#[macros::system]
pub fn update_animations(update: &Update, entities: Vec<Animations>) {
    for Animations { anim, tex, .. } in entities {
        anim.timer += update.0;
        if anim.timer >= anim.mspf {
            anim.frame = (anim.frame + anim.timer / anim.mspf) % anim.num_frames;
            anim.timer %= anim.mspf;

            let f = |rd: &mut RenderData| {
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
            };

            tex.try_mut(|rt: &mut RenderTexture| f(rt.get_render_data_mut()))
                .try_mut(tex, |ra: &mut RenderAsset| f(ra.get_render_data_mut()));
        }
    }
}
