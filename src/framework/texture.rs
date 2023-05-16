use std::{
    ffi::CString,
    ptr::{null_mut, NonNull},
};

use crate::{sdl2, sdl2_image, utils::rect::Dimensions};

use super::renderer::RendererTrait;

// Texture
pub trait TextureTrait {
    fn get(&self) -> *mut sdl2::SDL_Texture;

    fn set_blendmode(&self, mode: sdl2::SDL_BlendMode) {
        unsafe {
            sdl2::SDL_SetTextureBlendMode(self.get(), mode);
        }
    }

    fn get_size(&self) -> Dimensions<i32> {
        let mut d = Dimensions::<i32>::new();
        if unsafe {
            sdl2::SDL_QueryTexture(self.get(), null_mut(), null_mut(), &mut d.w, &mut d.h) != 0
        } {
            eprintln!("Failed to query texture")
        }
        d
    }
}

pub struct Texture {
    tex: NonNull<sdl2::SDL_Texture>,
}

impl Texture {
    pub fn new(tex: NonNull<sdl2::SDL_Texture>) -> Self {
        Self { tex }
    }

    pub fn from_file(r: &impl RendererTrait, file: &str) -> Self {
        let cstr = CString::new(file).expect("Failed to create CString");
        let t_ptr = unsafe { sdl2_image::IMG_LoadTexture(r.get(), cstr.as_ptr()) };
        Texture {
            tex: NonNull::new(t_ptr).expect("Failed to create Texture"),
        }
    }

    pub fn access(&self) -> TextureAccess {
        TextureAccess { tex: self.tex }
    }
}

impl TextureTrait for Texture {
    fn get(&self) -> *mut sdl2::SDL_Texture {
        self.tex.as_ptr()
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { sdl2::SDL_DestroyTexture(self.tex.as_ptr()) }
    }
}

// TextureAccess
// Non-owning, doesn't destroy
#[derive(Copy, Clone, Debug)]
pub struct TextureAccess {
    pub tex: NonNull<sdl2::SDL_Texture>,
}

impl TextureTrait for TextureAccess {
    fn get(&self) -> *mut sdl2::SDL_Texture {
        self.tex.as_ptr()
    }
}

// Helper traits
pub trait GetTexture {
    fn as_ptr(&self) -> *mut sdl2::SDL_Texture;
}

impl<T> GetTexture for Option<T>
where
    T: TextureTrait,
{
    fn as_ptr(&self) -> *mut sdl2::SDL_Texture {
        match self {
            Some(t) => t.get(),
            None => null_mut(),
        }
    }
}

// Track ownership
pub enum SharedTexture {
    Owned(Texture),
    Shared(TextureAccess),
    None,
}

impl SharedTexture {
    pub fn access(&self) -> Option<TextureAccess> {
        match self {
            SharedTexture::Owned(t) => Some(t.access()),
            SharedTexture::Shared(t) => Some(*t),
            SharedTexture::None => None,
        }
    }
}

impl From<Option<Texture>> for SharedTexture {
    fn from(value: Option<Texture>) -> Self {
        value.map_or(SharedTexture::None, |t| SharedTexture::Owned(t))
    }
}

impl From<Option<TextureAccess>> for SharedTexture {
    fn from(value: Option<TextureAccess>) -> Self {
        value.map_or(SharedTexture::None, |t| SharedTexture::Shared(t))
    }
}
