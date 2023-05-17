use crate::framework::texture::Texture;
use crate::sdl2;
use crate::utils::colors::BLACK;
use crate::utils::rect::Dimensions;
use crate::utils::rect::Rect;

use std::ffi::CString;
use std::ptr::null_mut;
use std::ptr::NonNull;

use super::texture::GetTexture;
use super::texture::TextureTrait;

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

    pub fn dimensions(self, width: u32, height: u32) -> Self {
        unsafe {
            sdl2::SDL_SetWindowSize(self.w.as_ptr(), width as i32, height as i32);
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
pub trait RendererTrait {
    fn get(&self) -> *mut sdl2::SDL_Renderer;

    // Managing render state
    fn set_target_ptr(&self, target: *mut sdl2::SDL_Texture) {
        unsafe { sdl2::SDL_SetRenderTarget(self.get(), target) };
    }

    fn set_target(&self, target: Option<impl TextureTrait>) {
        self.set_target_ptr(target.as_ptr())
    }

    fn clear_target(&self) {
        self.set_target_ptr(null_mut())
    }

    fn set_color(&self, color: sdl2::SDL_Color) {
        unsafe { sdl2::SDL_SetRenderDrawColor(self.get(), color.r, color.g, color.b, color.a) };
    }

    fn set_blendmode(&self, mode: sdl2::SDL_BlendMode) {
        unsafe { sdl2::SDL_SetRenderDrawBlendMode(self.get(), mode) };
    }

    // Get draw window size
    fn output_size(&self) -> Dimensions<i32> {
        let mut dim = Dimensions::<i32>::new();
        unsafe { sdl2::SDL_GetRendererOutputSize(self.get(), &mut dim.w, &mut dim.h) };
        dim
    }

    // Create new texture
    fn create_texture(&self, w: i32, h: i32) -> Option<Texture> {
        NonNull::new(unsafe {
            sdl2::SDL_CreateTexture(
                self.get(),
                sdl2::SDL_PixelFormatEnum::SDL_PIXELFORMAT_RGBA8888 as u32,
                sdl2::SDL_TextureAccess::SDL_TEXTUREACCESS_TARGET as i32,
                w,
                h,
            )
        })
        .map(|tex| {
            let tex = Texture::new(tex.as_ptr());
            tex.set_blendmode(sdl2::SDL_BlendMode::SDL_BLENDMODE_BLEND);
            tex
        })
    }

    // Drawing
    fn fill_rect(&self, rect: Rect) {
        unsafe { sdl2::SDL_RenderFillRect(self.get(), &rect.to_sdl_rect()) };
    }

    fn draw(
        &self,
        tex: &impl TextureTrait,
        src: *const sdl2::SDL_Rect,
        dest: *const sdl2::SDL_Rect,
    ) {
        unsafe {
            sdl2::SDL_RenderCopy(self.get(), tex.get(), src, dest);
        }
    }
}

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

    pub fn access(&self) -> RendererAccess {
        RendererAccess { r: self.r }
    }

    pub fn clear(&self) {
        self.set_color(BLACK);
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

impl RendererTrait for Renderer {
    fn get(&self) -> *mut sdl2::SDL_Renderer {
        self.r.as_ptr()
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe { sdl2::SDL_DestroyRenderer(self.get()) }
    }
}

// Non-owning Renderer
#[derive(Copy, Clone, Debug)]
pub struct RendererAccess {
    pub r: NonNull<sdl2::SDL_Renderer>,
}

impl RendererTrait for RendererAccess {
    fn get(&self) -> *mut sdl2::SDL_Renderer {
        self.r.as_ptr()
    }
}
