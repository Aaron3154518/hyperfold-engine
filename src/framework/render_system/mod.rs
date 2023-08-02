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
struct Elevation(pub u8);

pub enum Order {
    Asc,
    Desc,
}

pub fn sort_elevation<T>(
    mut arr: Vec<T>,
    get_elevation: impl for<'a> Fn(&'a T) -> &'a Elevation,
    get_eid: impl for<'a> Fn(&'a T) -> &'a Entity,
    asc: Order,
) -> Vec<T> {
    arr.sort_unstable_by(|t1, t2| {
        let (t1, t2) = match asc {
            Order::Asc => (t1, t2),
            Order::Desc => (t2, t1),
        };
        cmp([
            get_elevation(t1).0.cmp(&get_elevation(t2).0),
            get_eid(t1).cmp(get_eid(t2)),
        ])
    });
    arr
}

components!(RenderArgs, e: &'a Elevation, tex: &'a mut RenderComponent);

#[macros::system]
fn render(_e: &events::core::Render, comps: Vec<RenderArgs>, r: &Renderer, am: &mut AssetManager) {
    for RenderArgs { tex, .. } in sort_elevation(comps, |t| t.e, |t| t.eid, Order::Asc) {
        r.draw_asset(r, am, tex);
    }
}
