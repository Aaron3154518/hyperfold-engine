use crate::{
    sdl2, sdl2_image,
    utils::{
        colors::BLACK,
        rect::{Dimensions, Rect},
    },
};

use std::ptr::{null, NonNull};
use std::{ffi::CString, ptr::null_mut};

use super::{surface::Surface, Renderer, Texture, Window};

pub const W: u32 = 960;
pub const H: u32 = 720;

// Renderer
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
    fn set_target_ptr(&self, target: *mut sdl2::SDL_Texture) {
        unsafe { sdl2::SDL_SetRenderTarget(self.r.as_ptr(), target) };
    }

    pub fn set_target(&self, tex: &Texture) {
        self.set_target_ptr(tex.tex.as_ptr())
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
    pub fn output_size(&self) -> Dimensions<u32> {
        let (mut w, mut h) = (0, 0);
        unsafe { sdl2::SDL_GetRendererOutputSize(self.r.as_ptr(), &mut w, &mut h) };
        Dimensions {
            w: w as u32,
            h: h as u32,
        }
    }

    // Create new texture
    pub fn create_texture(&self, w: u32, h: u32) -> Option<Texture> {
        NonNull::new(unsafe {
            sdl2::SDL_CreateTexture(
                self.r.as_ptr(),
                sdl2::SDL_PixelFormatEnum::SDL_PIXELFORMAT_RGBA8888 as u32,
                sdl2::SDL_TextureAccess::SDL_TEXTUREACCESS_TARGET as i32,
                w as i32,
                h as i32,
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

    pub fn draw_texture(&self, tex: &Texture, src: Option<Rect>, dest: Option<Rect>) {
        let src = src.map(|r| r.to_sdl_rect());
        let dest = dest.map(|r| r.to_sdl_rect());

        unsafe {
            sdl2::SDL_RenderCopy(
                self.r.as_ptr(),
                tex.tex.as_ptr(),
                src.as_ref().map_or(null(), |r| r),
                dest.as_ref().map_or(null(), |r| r),
            )
        };
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe { sdl2::SDL_DestroyRenderer(self.r.as_ptr()) }
    }
}
