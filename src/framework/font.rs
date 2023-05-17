use std::{ffi::CString, ptr::NonNull};

use crate::{sdl2_ttf, utils::rect::Dimensions};

// Font

// Data for creating a font
pub const TIMES: &str = "res/fonts/times.ttf";

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontData {
    pub w: Option<u32>,
    pub h: Option<u32>,
    pub sample: String,
    pub file: String,
}

pub trait FontTrait {
    fn get(&self) -> *mut sdl2_ttf::TTF_Font;

    fn size(&self) -> Dimensions<i32> {
        let mut d = self.size_text("_");
        d.h = unsafe { sdl2_ttf::TTF_FontHeight(self.get()) };
        d
    }

    fn size_text(&self, text: &str) -> Dimensions<i32> {
        let cstr = CString::new(text).expect("Failed to create CString");
        let mut d = Dimensions::<i32>::new();
        unsafe { sdl2_ttf::TTF_SizeText(self.get(), cstr.as_ptr(), &mut d.w, &mut d.h) };
        d
    }
}

pub struct Font {
    font: NonNull<sdl2_ttf::TTF_Font>,
}

impl Font {
    pub fn new(font: NonNull<sdl2_ttf::TTF_Font>) -> Self {
        Self { font }
    }

    pub fn from_file(file: &str, size: u32) -> Self {
        let cstr = CString::new(file).expect("Failed to create CString");
        let f_ptr = unsafe { sdl2_ttf::TTF_OpenFont(cstr.as_ptr(), size as i32) };
        Self {
            font: NonNull::new(f_ptr).expect("Failed to create File"),
        }
    }

    pub fn access(&self) -> FontAccess {
        FontAccess { font: self.font }
    }
}

impl FontTrait for Font {
    fn get(&self) -> *mut sdl2_ttf::TTF_Font {
        self.font.as_ptr()
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe { sdl2_ttf::TTF_CloseFont(self.font.as_ptr()) };
    }
}

// Access
pub struct FontAccess {
    pub font: NonNull<sdl2_ttf::TTF_Font>,
}

impl FontTrait for FontAccess {
    fn get(&self) -> *mut sdl2_ttf::TTF_Font {
        self.font.as_ptr()
    }
}
