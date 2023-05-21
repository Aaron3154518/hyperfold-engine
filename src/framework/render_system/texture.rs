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

    pub fn get_size(&self) -> Dimensions<i32> {
        let mut d = Dimensions::<i32>::new();
        if unsafe {
            sdl2::SDL_QueryTexture(
                self.tex.as_ptr(),
                null_mut(),
                null_mut(),
                &mut d.w,
                &mut d.h,
            ) != 0
        } {
            eprintln!("Failed to query texture")
        }
        d
    }

    pub fn min_rect(&self, rect: Rect) -> Rect {
        let Dimensions::<i32> { w, h } = self.get_size();
        rect.get_fit_within(w as f32, h as f32)
    }

    pub fn min_rect_align(&self, rect: Rect, ax: Align, ay: Align) -> Rect {
        let mut r = self.min_rect(rect);
        r.copy_pos(rect, ax, ay);
        r
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { sdl2::SDL_DestroyTexture(self.tex.as_ptr()) }
    }
}
