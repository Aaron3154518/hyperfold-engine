use std::{ptr::NonNull, ffi::CString};

use crate::sdl2;

use super::Window;

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
