use std::cmp::Ordering;
use std::collections::HashMap;

use super::physics;
use crate::ecs::components::Container;
use crate::ecs::entities::Entity;
use crate::ecs::events;
use crate::sdl2;
use crate::utils::pointers::{self, Renderer, Texture, TextureAccess, TextureTrait, Window};
use crate::utils::rect::{Align, Dimensions, Rect};

const W: u32 = 960;
const H: u32 = 720;

pub struct AssetManager {
    file_imgs: HashMap<&'static str, Texture>,
}

impl AssetManager {
    pub fn new() -> Self {
        AssetManager {
            file_imgs: HashMap::new(),
        }
    }

    pub fn get_image(&self, file: &'static str) -> Option<TextureAccess> {
        match self.file_imgs.get(file) {
            Some(tex) => Some(tex.access()),
            None => None,
        }
    }

    pub fn add_image(&mut self, file: &'static str, tex: Texture) {
        self.file_imgs.insert(file, tex);
    }
}

#[macros::global]
pub struct RenderSystem {
    win: Window,
    pub r: Renderer,
    pub am: AssetManager,
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

    pub fn get_image(&mut self, file: &'static str) -> Option<TextureAccess> {
        match self.am.get_image(file) {
            Some(tex) => Some(tex),
            None => {
                self.am.add_image(file, Texture::new(&self.r, file));
                match self.am.get_image(file) {
                    Some(tex) => Some(tex),
                    None => {
                        println!("RenderSystem::get_image() - Unable to open file {}", file);
                        None
                    }
                }
            }
        }
    }

    pub fn draw(
        &self,
        tex: &TextureAccess,
        src: *const sdl2::SDL_Rect,
        dest: *const sdl2::SDL_Rect,
    ) {
        tex.draw(&self.r, src, dest);
    }
}

#[macro_export]
macro_rules! draw {
    ($rs: expr, $tex: ident, $src: expr, $dest: expr) => {
        if Some(tex) = $tex {
            $rs.draw(&tex, $src, $dest);
        }
    };
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
struct Image(pub Option<pointers::TextureAccess>);

#[macros::system]
fn render(
    _e: &events::core::Render,
    mut comps: Container<(&Entity, &mut Elevation, &Entity, &physics::Position, &Image)>,
    rs: &RenderSystem,
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
    for (_, _, _, pos, img) in comps {
        if let Image(Some(tex)) = img {
            rs.draw(
                &tex,
                std::ptr::null(),
                &rect_to_camera_coords(&pos.0, screen, camera).to_sdl_rect(),
            )
        }
    }
}
