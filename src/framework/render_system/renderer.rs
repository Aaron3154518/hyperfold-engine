use crate::{
    sdl2, sdl2_image,
    utils::{
        colors::BLACK,
        rect::{Dimensions, Rect},
    },
};

use super::surface::Surface;

use std::ptr::NonNull;
use std::{ffi::CString, ptr::null_mut};

use super::{Renderer, Texture, Window};

// Renderer
const W: u32 = 960;
const H: u32 = 720;

impl Renderer {
    pub fn new() -> Self {
        let win = Window::new().title("Game Engine").dimensions(W, H);
        Renderer {
            r: NonNull::new(unsafe { sdl2::SDL_CreateRenderer(win.w.as_ptr(), -1, 0) })
                .expect("Failed to create renderer"),
            win,
        }
    }

    pub fn clear(&self) {
        self.set_color(BLACK);
        unsafe { sdl2::SDL_RenderClear(self.r.as_ptr()) };
    }

    pub fn present(&self) {
        unsafe { sdl2::SDL_RenderPresent(self.r.as_ptr()) };
    }

    // Managing render state
    // TODO: dangling pointer here
    pub fn set_target_ptr(&self, target: *mut sdl2::SDL_Texture) {
        unsafe { sdl2::SDL_SetRenderTarget(self.r.as_ptr(), target) };
    }

    pub fn clear_target(&self) {
        self.set_target_ptr(null_mut())
    }

    pub fn set_color(&self, color: sdl2::SDL_Color) {
        unsafe {
            sdl2::SDL_SetRenderDrawColor(self.r.as_ptr(), color.r, color.g, color.b, color.a)
        };
    }

    pub fn set_blendmode(&self, mode: sdl2::SDL_BlendMode) {
        unsafe { sdl2::SDL_SetRenderDrawBlendMode(self.r.as_ptr(), mode) };
    }

    // Get draw window size
    pub fn output_size(&self) -> Dimensions<i32> {
        let mut dim = Dimensions::<i32>::new();
        unsafe { sdl2::SDL_GetRendererOutputSize(self.r.as_ptr(), &mut dim.w, &mut dim.h) };
        dim
    }

    // Create new texture
    pub fn create_texture(&self, w: i32, h: i32) -> Option<Texture> {
        NonNull::new(unsafe {
            sdl2::SDL_CreateTexture(
                self.r.as_ptr(),
                sdl2::SDL_PixelFormatEnum::SDL_PIXELFORMAT_RGBA8888 as u32,
                sdl2::SDL_TextureAccess::SDL_TEXTUREACCESS_TARGET as i32,
                w,
                h,
            )
        })
        .map(|tex| {
            let tex = Texture::from(tex.as_ptr());
            tex.set_blendmode(sdl2::SDL_BlendMode::SDL_BLENDMODE_BLEND);
            tex
        })
    }

    pub fn create_texture_from_file(&self, file: &str) -> Texture {
        let cstr = CString::new(file).expect("Failed to create CString");
        let t_ptr = unsafe { sdl2_image::IMG_LoadTexture(self.r.as_ptr(), cstr.as_ptr()) };
        Texture::from(t_ptr)
    }

    pub fn create_texture_from_surface(&self, surf: Surface) -> Texture {
        Texture::from(unsafe { sdl2::SDL_CreateTextureFromSurface(self.r.as_ptr(), surf.get()) })
    }

    // Drawing
    pub fn fill_rect(&self, rect: Rect) {
        unsafe { sdl2::SDL_RenderFillRect(self.r.as_ptr(), &rect.to_sdl_rect()) };
    }

    pub fn draw_line(&self, x1: i32, y1: i32, x2: i32, y2: i32) {
        unsafe { sdl2::SDL_RenderDrawLine(self.r.as_ptr(), x1, y1, x2, y2) };
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe { sdl2::SDL_DestroyRenderer(self.r.as_ptr()) }
    }
}
