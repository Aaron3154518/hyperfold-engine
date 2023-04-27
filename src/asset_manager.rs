use crate::sdl2;
use crate::utils::pointers::*;

use std::collections::HashMap;

use ecs_lib;

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

#[ecs_lib::component(Global)]
pub struct RenderSystem {
    win: Window,
    pub r: Renderer,
    pub am: AssetManager,
}

impl RenderSystem {
    pub fn new() -> Self {
        let w = 960;
        let h = 720;
        let win = Window::new().title("Game Engine").dimensions(w, h);
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
