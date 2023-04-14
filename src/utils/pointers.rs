use crate::sdl2;
use crate::sdl2_image;

use std::ffi::CString;
use std::ptr::NonNull;

// Window
pub struct Window {
    w: NonNull<sdl2::SDL_Window>,
}

impl Window {
    pub fn new() -> Self {
        let w_ptr = unsafe {
            sdl2::SDL_CreateWindow(
                CString::default().as_ptr(),
                sdl2::SDL_WINDOWPOS_CENTERED_MASK as i32,
                sdl2::SDL_WINDOWPOS_CENTERED_MASK as i32,
                640,
                480,
                sdl2::SDL_WindowFlags::SDL_WINDOW_SHOWN as u32,
            )
        };
        Window {
            w: NonNull::new(w_ptr).expect("Failed to create window"),
        }
    }

    pub fn title(self, title: &str) -> Self {
        let cstr = CString::new(title).expect("Failed to creat CString");
        unsafe {
            sdl2::SDL_SetWindowTitle(self.w.as_ptr(), cstr.as_ptr());
        }
        self
    }

    pub fn dimensions(self, width: i32, height: i32) -> Self {
        unsafe {
            sdl2::SDL_SetWindowSize(self.w.as_ptr(), width, height);
        }
        self
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe { sdl2::SDL_DestroyWindow(self.w.as_ptr()) }
    }
}

// Renderer
pub struct Renderer {
    r: NonNull<sdl2::SDL_Renderer>,
}

impl Renderer {
    pub fn new(win: &Window) -> Self {
        let r_ptr = unsafe { sdl2::SDL_CreateRenderer(win.w.as_ptr(), -1, 0) };
        Renderer {
            r: NonNull::new(r_ptr).expect("Failed to create renderer"),
        }
    }

    pub fn clear(&self) {
        unsafe {
            sdl2::SDL_RenderClear(self.r.as_ptr());
        }
    }

    pub fn present(&self) {
        unsafe {
            sdl2::SDL_RenderPresent(self.r.as_ptr());
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe { sdl2::SDL_DestroyRenderer(self.r.as_ptr()) }
    }
}

// Texture
pub trait TextureTrait {
    fn draw(&self, r: &Renderer, src: *const sdl2::SDL_Rect, dest: *const sdl2::SDL_Rect);
}

pub struct Texture {
    tex: NonNull<sdl2::SDL_Texture>,
}

impl Texture {
    pub fn new(r: &Renderer, file: &str) -> Self {
        let cstr = CString::new(file).expect("Failed to creat CString");
        let t_ptr = unsafe { sdl2_image::IMG_LoadTexture(r.r.as_ptr(), cstr.as_ptr()) };
        Texture {
            tex: NonNull::new(t_ptr).expect("Failed to create Texture"),
        }
    }

    pub fn access(&self) -> TextureAccess {
        TextureAccess { tex: self.tex }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { sdl2::SDL_DestroyTexture(self.tex.as_ptr()) }
    }
}

// TextureAccess
#[derive(Copy, Clone)]
pub struct TextureAccess {
    pub(crate) tex: NonNull<sdl2::SDL_Texture>,
}

impl TextureTrait for TextureAccess {
    fn draw(&self, r: &Renderer, src: *const sdl2::SDL_Rect, dest: *const sdl2::SDL_Rect) {
        unsafe {
            sdl2::SDL_RenderCopy(r.r.as_ptr(), self.tex.as_ptr(), src, dest);
        }
    }
}
