use std::ptr::{null_mut, NonNull};

use crate::{
    sdl2,
    utils::rect::{Align, Dimensions, Rect},
};

use super::Texture;

impl Texture {
    pub fn from(tex: *mut sdl2::SDL_Texture) -> Self {
        Self {
            tex: NonNull::new(tex).expect("Texture was null"),
        }
    }

    pub fn set_blendmode(&self, mode: sdl2::SDL_BlendMode) {
        unsafe {
            sdl2::SDL_SetTextureBlendMode(self.tex.as_ptr(), mode);
        }
    }

    pub fn get_size(&self) -> Dimensions<u32> {
        let (mut w, mut h) = (0, 0);
        if unsafe {
            sdl2::SDL_QueryTexture(self.tex.as_ptr(), null_mut(), null_mut(), &mut w, &mut h) != 0
        } {
            eprintln!("Failed to query texture")
        }
        Dimensions {
            w: w as u32,
            h: h as u32,
        }
    }

    pub fn min_rect(&self, rect: Rect) -> Rect {
        let Dimensions::<u32> { w, h } = self.get_size();
        rect.get_fit_within(w as f32, h as f32)
    }

    pub fn min_rect_align(&self, rect: Rect, ax: Align, ay: Align) -> Rect {
        self.min_rect(rect).from_rect_pos(rect, ax, ay)
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { sdl2::SDL_DestroyTexture(self.tex.as_ptr()) }
    }
}
