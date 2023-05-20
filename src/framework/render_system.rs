use std::cmp::Ordering;

use super::{
    asset_manager::{Asset, AssetManager, AssetManagerTrait},
    physics,
    renderer::RendererTrait,
};

use crate::{
    ecs::{components::Container, entities::Entity, events},
    framework::{
        renderer::{Renderer, Window},
        texture::Texture,
    },
    sdl2,
    utils::{
        colors::BLACK,
        rect::{Align, Dimensions, Rect},
    },
};

const W: u32 = 960;
const H: u32 = 720;

// RenderSystem
pub trait RenderSystemTrait {
    fn render_system_mut<'a>(&'a mut self) -> &'a mut RenderSystem;

    fn load_file<'a>(&'a mut self, file: &String) -> &'a Texture {
        let rs = self.render_system_mut();
        rs.am.get_or_load_asset_by_file(file, &rs.r)
    }

    fn load_asset<'a>(&'a mut self, asset: &'a Asset) -> Option<&'a Texture> {
        self.load_asset_then(asset, |t, _| t)
    }

    fn load_asset_then<'a, F, T>(&'a mut self, asset: &'a Asset, f: F) -> T
    where
        F: FnOnce(Option<&'a Texture>, &'a Renderer) -> T,
    {
        let rs = self.render_system_mut();
        f(
            match asset {
                Asset::File(file) => Some(rs.am.get_or_load_asset_by_file(file, &rs.r)),
                Asset::Id(id) => rs.am.get_asset_by_id(*id),
                Asset::Texture(tex) => Some(tex),
            },
            &rs.r,
        )
    }

    fn draw_asset(
        &mut self,
        asset: &Asset,
        src: *const sdl2::SDL_Rect,
        dest: *const sdl2::SDL_Rect,
    ) {
        self.load_asset_then(asset, |tex, r| {
            if let Some(tex) = tex {
                r.draw_texture(tex, src, dest)
            }
        })
    }
}

#[macros::global]
pub struct RenderSystem {
    win: Window,
    r: Renderer,
    am: AssetManager,
}

impl RenderSystem {
    pub fn new() -> Self {
        let win = Window::new().title("Game Engine").dimensions(W, H);
        let r = Renderer::new(&win);
        RenderSystem {
            win,
            r,
            am: AssetManager::new(),
        }
    }

    pub fn clear(&self) {
        self.set_color(BLACK);
        unsafe {
            sdl2::SDL_RenderClear(self.renderer());
        }
    }

    pub fn present(&self) {
        unsafe {
            sdl2::SDL_RenderPresent(self.renderer());
        }
    }
}

impl RendererTrait for RenderSystem {
    fn renderer(&self) -> *mut sdl2::SDL_Renderer {
        self.r.r.as_ptr()
    }
}

impl AssetManagerTrait for RenderSystem {
    fn asset_manager<'a>(&'a self) -> &'a AssetManager {
        &self.am
    }

    fn asset_manager_mut<'a>(&'a mut self) -> &'a mut AssetManager {
        &mut self.am
    }
}

impl RenderSystemTrait for RenderSystem {
    fn render_system_mut<'a>(&'a mut self) -> &'a mut RenderSystem {
        self
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

#[macros::component]
struct Image(pub Asset);

#[macros::system]
fn render(
    _e: &events::core::Render,
    mut comps: Container<(&Entity, &mut Elevation, &physics::Position, &Image)>,
    rs: &mut RenderSystem,
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
        rs.draw_asset(
            &img.0,
            std::ptr::null(),
            &rect_to_camera_coords(&pos.0, screen, camera).to_sdl_rect(),
        )
    }
}
