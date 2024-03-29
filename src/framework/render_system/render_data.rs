use uuid::Uuid;

use crate::{
    components,
    ecs::events::core::Update,
    impl_as_any,
    sdl2::SDL_RendererFlip,
    utils::{
        rect::{Align, Dimensions, Point, Rect},
        util::{AsAny, AsType, TryAsType},
    },
};

use super::{
    drawable::{AssetDrawable, Drawable, MutateDrawable},
    renderer::RenderOptions,
    Asset, AssetManager, Renderer, Texture,
};

// RenderData

#[derive(Copy, Clone, Debug)]
pub enum RectMode {
    Absolute,
    Percent,
}

#[derive(Copy, Clone, Debug)]
pub enum Fit {
    // Constrain to W and/or H
    Fit(bool, bool),
    // Keep texture dimensions
    None,
}

impl Fit {
    pub fn fit_dest() -> Self {
        Self::Fit(true, true)
    }

    pub fn fit_width() -> Self {
        Self::Fit(true, false)
    }

    pub fn fit_height() -> Self {
        Self::Fit(false, true)
    }

    pub fn fill_dest() -> Self {
        Self::Fit(false, false)
    }

    pub fn keep_dim() -> Self {
        Self::None
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Destination {
    pub(super) rect: Rect,
    pub(super) mode: RectMode,
    pub(super) fit: Fit,
    pub(super) align_x: Align,
    pub(super) align_y: Align,
}

impl Destination {
    pub fn to_rect(&self, Dimensions { w, h }: Dimensions<f32>) -> Rect {
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
            Fit::Fit(fit_w, fit_h) => {
                Rect::min_rect(w, h, fit_w.then_some(r.w), fit_h.then_some(r.h))
            }
            Fit::None => Rect::from_dim(w, h),
        }
        .with_rect_pos(r, self.align_x, self.align_y)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RenderData {
    pub(super) dest: Destination,
    pub(super) dest_rect: Rect,
    pub(super) area: Option<Rect>,
    opts: Option<RenderOptions>,
    dim: Dimensions<u32>,
    alpha: u8,
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
            fit: Fit::fit_dest(),
            align_x: Align::Center,
            align_y: Align::Center,
        };
        Self {
            dest,
            dest_rect: dest.to_rect(Dimensions {
                w: dim.w as f32,
                h: dim.h as f32,
            }),
            area: None,
            opts: None,
            dim,
            alpha: 255,
        }
    }

    pub fn get_tex_dim(&self) -> Dimensions<f32> {
        self.area.as_ref().map(|r| r.dim()).unwrap_or(Dimensions {
            w: self.dim.w as f32,
            h: self.dim.h as f32,
        })
    }
}

pub trait RenderDataTrait {
    fn get_render_data<'a>(&'a self) -> &'a RenderData;

    fn get_render_data_mut<'a>(&'a mut self) -> &'a mut RenderData;

    fn get_dest_opts(&self) -> Destination {
        self.get_render_data().dest
    }

    fn get_render_opts(&self) -> Option<RenderOptions> {
        self.get_render_data().opts
    }

    fn set_alpha(&mut self, alpha: u8) {
        let rd = self.get_render_data_mut();
        rd.alpha = alpha;
    }

    fn set_dim(&mut self, dim: Dimensions<u32>) {
        let rd = self.get_render_data_mut();
        rd.dim = dim;
        rd.dest_rect = rd.dest.to_rect(rd.get_tex_dim());
    }

    fn set_dest(&mut self, dest: Destination) {
        let rd = self.get_render_data_mut();
        rd.dest = dest;
        rd.dest_rect = rd.dest.to_rect(rd.get_tex_dim());
    }

    fn set_dest_opts(
        &mut self,
        rect: Rect,
        mode: RectMode,
        fit: Fit,
        align_x: Align,
        align_y: Align,
    ) {
        self.set_dest(Destination {
            rect,
            mode,
            fit,
            align_x,
            align_y,
        });
    }

    fn set_dest_rect(&mut self, rect: Rect) {
        let rd = self.get_render_data_mut();
        rd.dest.rect = rect;
        rd.dest_rect = rd.dest.to_rect(rd.get_tex_dim());
    }

    fn set_dest_mode(&mut self, mode: RectMode) {
        let rd = self.get_render_data_mut();
        rd.dest.mode = mode;
        rd.dest_rect = rd.dest.to_rect(rd.get_tex_dim());
    }

    fn set_dest_fit(&mut self, fit: Fit) {
        let rd = self.get_render_data_mut();
        rd.dest.fit = fit;
        rd.dest_rect = rd.dest.to_rect(rd.get_tex_dim());
    }

    fn set_dest_align(&mut self, align_x: Align, align_y: Align) {
        let rd = self.get_render_data_mut();
        rd.dest.align_x = align_x;
        rd.dest.align_y = align_y;
        rd.dest_rect = rd.dest.to_rect(rd.get_tex_dim());
    }

    fn set_area(&mut self, area: Option<Rect>) {
        self.get_render_data_mut().area = area;
    }

    fn set_render_options(&mut self, opts: Option<RenderOptions>) {
        self.get_render_data_mut().opts = opts;
    }

    fn set_rotation(&mut self, deg: f64, center: Option<Point>) {
        let rd = self.get_render_data_mut();
        let opts = rd.opts.get_or_insert(RenderOptions::default());
        opts.rotation_deg = deg;
        opts.rotation_center = center;
    }

    fn set_flip(&mut self, flip: SDL_RendererFlip) {
        self.get_render_data_mut()
            .opts
            .get_or_insert(RenderOptions::default())
            .flip = flip;
    }

    fn animate(&mut self, anim: Animation) {
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
    }

    fn draw_texture(&self, r: &Renderer, tex: &Texture) {
        let rd = self.get_render_data();
        r.draw_texture(tex, rd.area, Some(rd.dest_rect), &rd.opts, rd.alpha);
    }
}

impl<T: RenderDataTrait> MutateDrawable for T {
    fn set_rect(&mut self, r: Rect) {
        self.get_render_data_mut().set_dest_rect(r);
    }
}

pub trait RenderDataBuilderTrait
where
    Self: Sized + RenderDataTrait,
{
    fn with_alpha(mut self, alpha: u8) -> Self {
        self.set_alpha(alpha);
        self
    }

    fn fill_dest(self) -> Self {
        self.with_dest_rect(Rect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        })
        .with_dest_mode(RectMode::Percent)
    }

    fn with_dest(mut self, dest: Destination) -> Self {
        self.set_dest(dest);
        self
    }

    fn with_dest_opts(
        mut self,
        rect: Rect,
        mode: RectMode,
        fit: Fit,
        align_x: Align,
        align_y: Align,
    ) -> Self {
        self.set_dest_opts(rect, mode, fit, align_x, align_y);
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

    fn with_dest_fit(mut self, fit: Fit) -> Self {
        self.set_dest_fit(fit);
        self
    }

    fn with_dest_align(mut self, align_x: Align, align_y: Align) -> Self {
        self.set_dest_align(align_x, align_y);
        self
    }

    fn with_area(mut self, area: Option<Rect>) -> Self {
        self.set_area(area);
        self
    }

    fn with_rotation(mut self, deg: f64, center: Option<Point>) -> Self {
        self.set_rotation(deg, center);
        self
    }

    fn with_flip(mut self, flip: SDL_RendererFlip) -> Self {
        self.set_flip(flip);
        self
    }

    fn with_animation(mut self, anim: Animation) -> Self {
        self.animate(anim);
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
            self.draw_texture(r, tex);
        }
    }
}

impl_as_any!(RenderTexture);

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

    pub fn from_file(file: &str, r: &Renderer, am: &mut AssetManager) -> Self {
        Self::new(Asset::File(file.to_string()), r, am)
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
            self.draw_texture(r, tex);
        }
    }
}

impl_as_any!(RenderAsset);

// RenderComponent
pub trait RenderComponentTrait: AssetDrawable + MutateDrawable + AsAny {}

impl<T> RenderComponentTrait for T where T: AssetDrawable + MutateDrawable + AsAny {}

#[macros::component]
struct RenderComponent(pub(super) Box<dyn RenderComponentTrait>);

impl RenderComponent {
    pub fn new(d: impl RenderComponentTrait + 'static) -> Self {
        Self(Box::new(d))
    }

    pub fn set(&mut self, d: impl RenderComponentTrait + 'static) {
        self.0 = Box::new(d)
    }
}

impl AsAny for RenderComponent {
    fn as_any<'a>(&'a self) -> &'a dyn std::any::Any {
        self.0.as_any()
    }

    fn as_any_mut<'a>(&'a mut self) -> &'a mut dyn std::any::Any {
        self.0.as_any_mut()
    }
}

impl AssetDrawable for RenderComponent {
    fn draw(&mut self, r: &Renderer, am: &mut AssetManager) {
        self.0.draw(r, am);
    }
}

// Animation
#[derive(Copy, Clone)]
#[macros::component]
struct Animation {
    num_frames: u32,
    frame: u32,
    mspf: u32,
    timer: u32,
    loop_anim: bool,
    running: bool,
}

impl Animation {
    fn create(num_frames: u32, mspf: u32, loop_anim: bool) -> Self {
        Self {
            num_frames,
            frame: 0,
            mspf,
            timer: 0,
            loop_anim,
            running: true,
        }
    }

    pub fn new(num_frames: u32, mspf: u32) -> Self {
        Self::create(num_frames, mspf, true)
    }

    pub fn once(num_frames: u32, mspf: u32) -> Self {
        Self::create(num_frames, mspf, false)
    }
}

components!(
    Animations,
    anim: &'a mut Animation,
    tex: &'a mut RenderComponent
);

#[macros::system]
pub fn update_animations(update: &Update, entities: Vec<Animations>) {
    for Animations { anim, tex, .. } in entities.into_iter().filter(|e| e.anim.running) {
        anim.timer += update.0;
        if anim.timer >= anim.mspf {
            anim.frame = (anim.frame + anim.timer / anim.mspf) % anim.num_frames;
            anim.timer %= anim.mspf;
            if !anim.loop_anim && anim.frame == anim.num_frames - 1 {
                anim.running = false;
            }

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

            tex.try_as_mut(|rt: &mut RenderTexture| f(rt.get_render_data_mut()))
                .try_as_mut(tex, |ra: &mut RenderAsset| f(ra.get_render_data_mut()));
        }
    }
}
