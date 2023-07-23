use crate::sdl2::{SDL_Color, Uint32};

// ARGB masks for creating surfaces and colors
pub const RMASK: Uint32 = 0xff000000;
pub const GMASK: Uint32 = 0x00ff0000;
pub const BMASK: Uint32 = 0x0000ff00;
pub const AMASK: Uint32 = 0x000000ff;

// Colors
pub fn gray(val: u8) -> SDL_Color {
    SDL_Color {
        r: val,
        g: val,
        b: val,
        a: 255,
    }
}

pub const TRANSPARENT: SDL_Color = SDL_Color {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};
pub const WHITE: SDL_Color = SDL_Color {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};
pub const LGRAY: SDL_Color = SDL_Color {
    r: 175,
    g: 175,
    b: 175,
    a: 255,
};
pub const GRAY: SDL_Color = SDL_Color {
    r: 128,
    g: 128,
    b: 128,
    a: 255,
};
pub const DGRAY: SDL_Color = SDL_Color {
    r: 64,
    g: 64,
    b: 64,
    a: 255,
};
pub const BLACK: SDL_Color = SDL_Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};
pub const RED: SDL_Color = SDL_Color {
    r: 255,
    g: 0,
    b: 0,
    a: 255,
};
pub const ORANGE: SDL_Color = SDL_Color {
    r: 255,
    g: 165,
    b: 0,
    a: 255,
};
pub const YELLOW: SDL_Color = SDL_Color {
    r: 255,
    g: 255,
    b: 0,
    a: 255,
};
pub const GREEN: SDL_Color = SDL_Color {
    r: 0,
    g: 255,
    b: 0,
    a: 255,
};
pub const CYAN: SDL_Color = SDL_Color {
    r: 0,
    g: 255,
    b: 255,
    a: 255,
};
pub const BLUE: SDL_Color = SDL_Color {
    r: 0,
    g: 0,
    b: 255,
    a: 255,
};
pub const MAGENTA: SDL_Color = SDL_Color {
    r: 255,
    g: 0,
    b: 255,
    a: 255,
};
pub const PURPLE: SDL_Color = SDL_Color {
    r: 128,
    g: 0,
    b: 128,
    a: 255,
};
