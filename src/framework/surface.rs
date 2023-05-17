use std::ptr::NonNull;

use crate::sdl2;

pub struct Surface {
    surf: NonNull<sdl2::SDL_Surface>,
}

impl Surface {
    pub fn new(surf: *mut sdl2::SDL_Surface) -> Self {
        Self {
            surf: NonNull::new(surf).expect("Surface was null"),
        }
    }

    pub fn get(&self) -> *mut sdl2::SDL_Surface {
        self.surf.as_ptr()
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe { sdl2::SDL_FreeSurface(self.get()) }
    }
}
