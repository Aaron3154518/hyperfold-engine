use std::cmp::Ordering;
use std::collections::HashMap;

use shared::util::NoneOr;
use uuid::Uuid;

use super::{
    font::{Font, FontAccess, FontData, FontTrait},
    physics,
    renderer::RendererTrait,
};
// use super::texture::SharedTexture;
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

pub enum Asset {
    File(String),
    Id(Uuid),
    Texture(Texture),
}

pub struct AssetManager {
    file_assets: HashMap<String, Texture>,
    id_assets: HashMap<Uuid, Texture>,
    fonts: HashMap<FontData, Font>,
}

impl AssetManager {
    pub fn new() -> Self {
        AssetManager {
            file_assets: HashMap::new(),
            id_assets: HashMap::new(),
            fonts: HashMap::new(),
        }
    }
}

impl AssetManagerTrait for AssetManager {
    fn asset_manager<'a>(&'a self) -> &'a AssetManager {
        self
    }

    fn asset_manager_mut<'a>(&'a mut self) -> &'a mut AssetManager {
        self
    }
}

pub trait AssetManagerTrait {
    fn asset_manager<'a>(&'a self) -> &'a AssetManager;
    fn asset_manager_mut<'a>(&'a mut self) -> &'a mut AssetManager;

    fn get_asset_by_file<'a>(&'a self, file: &String) -> Option<&'a Texture> {
        self.asset_manager().file_assets.get(file)
    }

    fn get_or_load_asset_by_file<'a>(
        &'a mut self,
        file: &String,
        r: &impl RendererTrait,
    ) -> &'a Texture {
        if self.get_asset_by_file(file).is_none() {
            let am = self.asset_manager_mut();
            am.file_assets
                .insert(file.to_string(), Texture::from_file(r, file));
            self.get_asset_by_file(file).expect("Failed to load asset")
        } else {
            self.get_asset_by_file(file).expect("File to get asset")
        }
    }

    fn get_asset_by_id<'a>(&'a self, id: Uuid) -> Option<&'a Texture> {
        self.asset_manager().id_assets.get(&id)
    }

    fn add_texture(&mut self, tex: Texture) -> Asset {
        let id = Uuid::new_v4();
        self.asset_manager_mut().id_assets.insert(id, tex);
        Asset::Id(id)
    }

    fn add_image<'a>(&'a mut self, file: &str, tex: Texture) -> Option<&'a Texture> {
        let am = self.asset_manager_mut();
        am.file_assets.insert(file.to_string(), tex);
        am.file_assets.get(&file.to_string())
    }

    fn get_font(&mut self, data: FontData) -> Option<FontAccess> {
        let am = self.asset_manager_mut();
        match am.fonts.get(&data) {
            Some(f) => Some(f.access()),
            None => {
                // Min is always too small or just right, max is too big
                let (mut min_size, mut max_size) = (1, 10);
                // If both dimensions are none, use smallest font
                if data.w.is_some() || data.h.is_some() {
                    let mut dim = Font::from_file(&data.file, min_size).size_text(&data.sample);
                    // While too small
                    while data.w.is_none_or(|w| dim.w as u32 <= *w)
                        && data.h.is_none_or(|h| dim.h as u32 <= *h)
                    {
                        min_size = max_size;
                        max_size *= 2;
                        dim = Font::from_file(&data.file, max_size).size_text(&data.sample);
                    }

                    // Terminate when max_size (too big) is right after min_size (too small)
                    while max_size - min_size > 1 {
                        let size = (max_size + min_size) / 2;
                        dim = Font::from_file(&data.file, size).size_text(&data.sample);
                        // Too big
                        if data.w.is_some_and(|w| dim.w as u32 > w)
                            || data.h.is_some_and(|h| dim.h as u32 > h)
                        {
                            max_size = size;
                        } else {
                            // Too small or just right
                            min_size = size;
                        }
                    }
                }

                let file = data.file.to_string();
                am.fonts
                    .try_insert(data, Font::from_file(&file, min_size))
                    .ok()
                    .map(|f| f.access())
            }
        }
    }
}

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
