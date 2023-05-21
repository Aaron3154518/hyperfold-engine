use std::{
    cmp::Ordering,
    collections::HashMap,
    ptr::{null, NonNull},
};

use uuid::Uuid;

use super::physics;
use font::{Font, FontData};

use crate::{
    ecs::{components::Container, entities::Entity, events},
    sdl2,
    utils::rect::{Align, Dimensions, Rect},
};

pub mod asset_manager;
pub mod drawable;
pub mod font;
pub mod render_data;
pub mod renderer;
pub mod shapes;
pub mod surface;
pub mod text;
pub mod texture;
pub mod window;

const W: u32 = 960;
const H: u32 = 720;

pub struct Texture {
    tex: NonNull<sdl2::SDL_Texture>,
}

pub struct Window {
    w: NonNull<sdl2::SDL_Window>,
}

#[macros::global]
pub struct Renderer {
    r: NonNull<sdl2::SDL_Renderer>,
    win: Window,
}

pub enum Asset {
    File(String),
    Id(Uuid),
}

#[macros::global]
pub struct AssetManager {
    file_assets: HashMap<String, Texture>,
    id_assets: HashMap<Uuid, Texture>,
    fonts: HashMap<FontData, Font>,
}

// Renderer and Texture
pub trait TextureRendererTrait {
    fn renderer<'a>(&'a self) -> &'a Renderer;
    fn texture<'a>(&'a self) -> &'a Texture;

    fn set_target(&self) {
        self.renderer().set_target_ptr(self.texture().tex.as_ptr())
    }

    fn draw_texture(&self, src: *const sdl2::SDL_Rect, dest: *const sdl2::SDL_Rect) {
        unsafe {
            sdl2::SDL_RenderCopy(
                self.renderer().r.as_ptr(),
                self.texture().tex.as_ptr(),
                src,
                dest,
            );
        }
    }
}

impl TextureRendererTrait for (&Renderer, &Texture) {
    fn renderer<'a>(&'a self) -> &'a Renderer {
        &self.0
    }

    fn texture<'a>(&'a self) -> &'a Texture {
        &self.1
    }
}

impl TextureRendererTrait for (&Texture, &Renderer) {
    fn renderer<'a>(&'a self) -> &'a Renderer {
        &self.1
    }

    fn texture<'a>(&'a self) -> &'a Texture {
        &self.0
    }
}

// Renderer and AssetManager
pub trait RenderSystemTrait {
    fn get_data<'a>(&'a mut self) -> (&'a Renderer, &'a mut AssetManager);

    fn load_file<'a>(&'a mut self, file: &String) -> &'a Texture {
        let (r, am) = self.get_data();
        // let r = self.renderer();
        // let am = self.asset_manager();
        am.get_or_load_asset_by_file(file, r)
    }

    fn load_asset<'a>(&'a mut self, asset: &'a Asset) -> Option<&'a Texture> {
        let (r, am) = self.get_data();
        match asset {
            Asset::File(file) => Some(am.get_or_load_asset_by_file(file, r)),
            Asset::Id(id) => am.get_asset_by_id(*id),
        }
    }

    fn draw_asset(
        &mut self,
        asset: &Asset,
        src: *const sdl2::SDL_Rect,
        dest: *const sdl2::SDL_Rect,
    ) {
        let (r, am) = self.get_data();
        (
            r,
            match asset {
                Asset::File(file) => am.get_or_load_asset_by_file(file, r),
                Asset::Id(id) => match am.get_asset_by_id(*id) {
                    Some(t) => t,
                    None => return,
                },
            },
        )
            .draw_texture(src, dest)
    }
}

impl RenderSystemTrait for (&Renderer, &mut AssetManager) {
    fn get_data<'a>(&'a mut self) -> (&'a Renderer, &'a mut AssetManager) {
        (&self.0, &mut self.1)
    }
}

impl RenderSystemTrait for (&mut AssetManager, &Renderer) {
    fn get_data<'a>(&'a mut self) -> (&'a Renderer, &'a mut AssetManager) {
        (&self.1, &mut self.0)
    }
}

#[macros::global(Const)]
pub struct Screen(pub Dimensions<u32>);

impl Screen {
    pub fn new() -> Self {
        Self(Dimensions { w: W, h: H })
    }
}

#[macros::global]
pub struct Camera(pub Rect);

impl Camera {
    pub fn new() -> Self {
        Self(Rect {
            x: 0.0,
            y: 0.0,
            w: W as f32,
            h: H as f32,
        })
    }
}

pub fn rect_to_camera_coords(rect: &Rect, screen: &Screen, camera: &Camera) -> Rect {
    let w_frac = screen.0.w as f32 / camera.0.w();
    let h_frac = screen.0.h as f32 / camera.0.h();
    let mut r = Rect {
        x: 0.0,
        y: 0.0,
        w: rect.w() * w_frac,
        h: rect.h() * h_frac,
    };
    r.set_pos(
        (rect.cx() - camera.0.x()) * w_frac,
        (rect.cy() - camera.0.y()) * h_frac,
        Align::Center,
        Align::Center,
    );
    r
}

#[macros::component]
struct Elevation(pub u8);

pub enum TextureAccess {
    Asset(Asset),
    Texture(Texture),
}

#[macros::component]
struct Image(pub TextureAccess);

impl Image {
    pub fn from_file(file: String) -> Self {
        Self(TextureAccess::Asset(Asset::File(file)))
    }

    pub fn from_id(id: Uuid) -> Self {
        Self(TextureAccess::Asset(Asset::Id(id)))
    }

    pub fn from_texture(tex: Texture) -> Self {
        Self(TextureAccess::Texture(tex))
    }
}

#[macros::system]
fn render(
    _e: &events::core::Render,
    mut comps: Container<(&Entity, &mut Elevation, &physics::Position, &Image)>,
    r: &Renderer,
    am: &mut AssetManager,
    screen: &Screen,
    camera: &Camera,
) {
    comps.sort_by(|(id1, e1, ..), (id2, e2, ..)| {
        let cmp = e1.0.cmp(&e2.0);
        if cmp == Ordering::Equal {
            id1.cmp(&id2)
        } else {
            cmp
        }
    });
    for (_, _, pos, img) in comps {
        let pos = &rect_to_camera_coords(&pos.0, screen, camera).to_sdl_rect();
        match &img.0 {
            TextureAccess::Asset(a) => (r, &mut *am).draw_asset(&a, null(), pos),
            TextureAccess::Texture(t) => (r, t).draw_texture(null(), pos),
        }
    }
}
