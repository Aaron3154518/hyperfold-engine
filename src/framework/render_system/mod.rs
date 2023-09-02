use std::{collections::HashMap, ptr::NonNull};

use uuid::Uuid;

use self::drawable::Canvas;

use font::{Font, FontData};
pub use render_data::RenderComponent;

use crate::{
    components,
    ecs::{entities::Entity, events},
    sdl2,
    utils::{
        rect::{Align, Dimensions, Rect},
        util::cmp,
    },
};

use super::physics::Position;

pub mod asset_manager;
pub mod drawable;
pub mod font;
pub mod render_data;
pub mod render_text;
pub mod renderer;
pub mod shapes;
pub mod surface;
pub mod text;
pub mod texture;
pub mod window;

const W: u32 = 960;
const H: u32 = 720;

// RenderSystem structs
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

// Globals/Components
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

#[derive(Clone, Copy)]
#[macros::component]
struct RenderOpts {
    pub elevation: u8,
    pub absolute: bool,
    pub visible: bool,
}

impl RenderOpts {
    pub fn new(e: u8) -> Self {
        Self {
            elevation: e,
            absolute: false,
            visible: true,
        }
    }

    pub fn set_absolute(&mut self, abs: bool) {
        self.absolute = abs;
    }

    pub fn is_absolute(mut self, abs: bool) -> Self {
        self.set_absolute(abs);
        self
    }

    pub fn absolute(self) -> Self {
        self.is_absolute(true)
    }

    pub fn set_visibility(&mut self, vis: bool) {
        self.visible = vis;
    }

    pub fn with_visibility(mut self, vis: bool) -> Self {
        self.set_visibility(vis);
        self
    }
}

pub enum Order {
    Asc,
    Desc,
}

pub fn sort_elevation<T>(
    mut arr: Vec<T>,
    get_opts: impl for<'a> Fn(&'a T) -> &'a RenderOpts,
    get_eid: impl for<'a> Fn(&'a T) -> &'a Entity,
    asc: Order,
) -> Vec<T> {
    arr.sort_unstable_by(|t1, t2| {
        let (t1, t2) = match asc {
            Order::Asc => (t1, t2),
            Order::Desc => (t2, t1),
        };
        cmp([
            get_opts(t1).elevation.cmp(&get_opts(t2).elevation),
            get_eid(t1).cmp(get_eid(t2)),
        ])
    });
    arr
}

components!(RenderArgs, opts: &'a RenderOpts, tex: &'a mut RenderComponent, pos: &'a Position);

#[macros::system]
fn render(
    _e: &events::core::Render,
    comps: Vec<RenderArgs>,
    r: &Renderer,
    am: &mut AssetManager,
    screen: &Screen,
    camera: &Camera,
) {
    for RenderArgs { tex, opts, pos, .. } in
        sort_elevation(comps, |t| t.opts, |t| t.eid, Order::Asc)
            .into_iter()
            .filter(|r| r.opts.visible)
    {
        tex.0.set_rect(match opts.absolute {
            true => pos.0,
            false => rect_to_camera_coords(&pos.0, screen, camera),
        });
        r.draw_asset(r, am, tex);
    }
}
