use num_traits::FromPrimitive;

use super::rect::*;
use crate::sdl2;
use std::{collections::HashMap, mem};

const NUM_MICE: u8 = 3;
const MAX_CLICK_DIFF: f32 = 10.0;
const MOUSE_BTN_LEFT: u8 = sdl2::SDL_BUTTON_LEFT as u8;
const MOUSE_BTN_RIGHT: u8 = sdl2::SDL_BUTTON_RIGHT as u8;
const MOUSE_BTN_MIDDLE: u8 = sdl2::SDL_BUTTON_MIDDLE as u8;

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Mouse {
    Left = 0,
    Right,
    Middle,
}

impl Mouse {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            MOUSE_BTN_LEFT => Some(Mouse::Left),
            MOUSE_BTN_RIGHT => Some(Mouse::Right),
            MOUSE_BTN_MIDDLE => Some(Mouse::Middle),
            _ => None,
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Status {
    Down = 0x01,
    Up = 0x02,
    Held = 0x04,
    Pressed = 0x08,
}

#[derive(Copy, Clone)]
pub struct MouseButton {
    pub mouse: Mouse,
    pub click_pos: Point,
    pub duration: u32,
    status: u8,
}

impl MouseButton {
    fn new(mouse: Mouse) -> Self {
        Self {
            mouse,
            click_pos: Point { x: 0, y: 0 },
            duration: 0,
            status: 0,
        }
    }

    pub fn down(&self) -> bool {
        self.status & Status::Down as u8 != 0
    }

    pub fn up(&self) -> bool {
        self.status & Status::Up as u8 != 0
    }

    pub fn held(&self) -> bool {
        self.status & Status::Held as u8 != 0
    }

    pub fn clicked(&self) -> bool {
        self.status & Status::Pressed as u8 != 0
    }
}

#[derive(Copy, Clone)]
pub struct KeyButton {
    pub key: sdl2::SDL_KeyCode,
    pub duration: u32,
    status: u8,
}

impl KeyButton {
    fn new(key: sdl2::SDL_KeyCode) -> Self {
        Self {
            key: key,
            duration: 0,
            status: 0,
        }
    }

    pub fn down(&self) -> bool {
        self.status & Status::Down as u8 != 0
    }

    pub fn up(&self) -> bool {
        self.status & Status::Up as u8 != 0
    }

    pub fn held(&self) -> bool {
        self.status & Status::Held as u8 != 0
    }

    pub fn pressed(&self) -> bool {
        self.status & Status::Pressed as u8 != 0
    }
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum InputSeek {
    None = 0,
    Start,
    End,
}

pub struct Event {
    pub dt: u32,
    pub quit: bool,
    pub resized: bool,
    pub old_dim: Dimensions,
    pub new_dim: Dimensions,
    pub mouse: Point,
    pub abs_mouse: Point,
    pub mouse_delta: Point,
    pub scroll: i32,
    pub input_text: String,
    pub input_backspace: i32,
    pub input_delete: i32,
    pub input_move: i32,
    pub input_seek: InputSeek,
    pub mouse_buttons: [MouseButton; NUM_MICE as usize],
    pub key_buttons: HashMap<sdl2::SDL_KeyCode, KeyButton>,
}

impl Event {
    pub fn new() -> Self {
        Self {
            dt: 0,
            quit: false,
            resized: false,
            old_dim: Dimensions { w: 0, h: 0 },
            new_dim: Dimensions { w: 0, h: 0 },
            mouse: Point { x: 0, y: 0 },
            abs_mouse: Point { x: 0, y: 0 },
            mouse_delta: Point { x: 0, y: 0 },
            scroll: 0,
            input_text: "".to_string(),
            input_backspace: 0,
            input_delete: 0,
            input_move: 0,
            input_seek: InputSeek::None,
            mouse_buttons: [
                MouseButton::new(Mouse::Left),
                MouseButton::new(Mouse::Right),
                MouseButton::new(Mouse::Middle),
            ],
            key_buttons: HashMap::new(),
        }
    }

    pub fn update(&mut self, ts: u32, camera: &Rect, screen: &Dimensions) {
        self.dt = ts;
        // Reset event flags
        self.quit = false;
        self.resized = false;
        // Reset text editing
        self.input_text.clear();
        self.input_backspace = 0;
        self.input_delete = 0;
        self.input_move = 0;
        self.input_seek = InputSeek::None;
        // Update mouse
        let (mut x, mut y) = (0, 0);
        unsafe {
            sdl2::SDL_GetMouseState(&mut x, &mut y);
        }
        self.abs_mouse = Point { x, y };
        self.mouse = Point {
            x: (x as f32 * camera.w() / screen.w as f32 + camera.x()) as i32,
            y: (y as f32 * camera.h() / screen.h as f32 + camera.y()) as i32,
        };
        // Reset mouse movement
        self.mouse_delta = Point { x: 0, y: 0 };
        self.scroll = 0;
        // Update mouse buttons
        for b in &mut self.mouse_buttons {
            if b.held() {
                b.duration += ts;
            }
            // Reset pressed/released
            b.status &= Status::Held as u8;
        }
        // Update keys
        for (_, b) in &mut self.key_buttons {
            if b.held() {
                b.duration += ts;
            }
            // Reset pressed/released
            b.status &= Status::Held as u8;
        }
        // Handle events
        let mut event = unsafe { mem::zeroed() };
        while unsafe { sdl2::SDL_PollEvent(&mut event) } != 0 {
            self.update_event(&event);
        }
    }

    fn update_event(&mut self, event: &sdl2::SDL_Event) {
        match FromPrimitive::from_u32(unsafe { event.type_ }) {
            Some(sdl2::SDL_EventType::SDL_QUIT) => {
                self.quit = true;
            }
            Some(sdl2::SDL_EventType::SDL_WINDOWEVENT) => match FromPrimitive::from_u8(unsafe {
                event.window.event
            }) {
                Some(sdl2::SDL_WindowEventID::SDL_WINDOWEVENT_SHOWN) => {
                    unsafe {
                        let window = sdl2::SDL_GetWindowFromID(event.window.windowID);
                        sdl2::SDL_GetWindowSize(window, &mut self.old_dim.w, &mut self.old_dim.h);
                    }
                    self.new_dim = self.old_dim;
                }
                Some(sdl2::SDL_WindowEventID::SDL_WINDOWEVENT_RESIZED) => {
                    self.resized = true;
                    self.old_dim = self.new_dim;
                    self.new_dim = Dimensions {
                        w: unsafe { event.window.data1 },
                        h: unsafe { event.window.data2 },
                    };
                }
                _ => {}
            },
            Some(sdl2::SDL_EventType::SDL_MOUSEBUTTONDOWN) => {
                match Mouse::from_u8(unsafe { event.button.button }) {
                    Some(b) => {
                        let button = &mut self.mouse_buttons[b as usize];
                        button.status = Status::Down as u8 | Status::Held as u8;
                        button.duration = 0;
                        button.click_pos = self.mouse;
                    }
                    None => {}
                }
            }
            Some(sdl2::SDL_EventType::SDL_MOUSEBUTTONUP) => {
                match Mouse::from_u8(unsafe { event.button.button }) {
                    Some(b) => {
                        let button = &mut self.mouse_buttons[b as usize];
                        let max_click_diff = MAX_CLICK_DIFF;
                        button.status = if button.click_pos.dist(&self.mouse) < max_click_diff {
                            Status::Pressed as u8 | Status::Up as u8
                        } else {
                            Status::Up as u8
                        };
                        button.duration = 0;
                    }
                    None => {}
                }
            }
            Some(sdl2::SDL_EventType::SDL_MOUSEMOTION) => {
                self.mouse_delta = Point {
                    x: unsafe { event.motion.xrel },
                    y: unsafe { event.motion.yrel },
                };
            }
            Some(sdl2::SDL_EventType::SDL_MOUSEWHEEL) => {
                self.scroll = -unsafe { event.wheel.y };
            }
            Some(sdl2::SDL_EventType::SDL_KEYDOWN) => {
                match FromPrimitive::from_i32(unsafe { event.key.keysym.sym }) {
                    Some(k) => {
                        let b = self.get_key_mut(k);
                        let held = b.held();
                        b.status = Status::Pressed as u8 | Status::Held as u8;
                        if !held {
                            b.status |= Status::Down as u8;
                            b.duration = 0;
                        }
                        if unsafe { sdl2::SDL_IsTextInputActive() } == sdl2::SDL_bool::SDL_TRUE {
                            self.process_text_input_key(k);
                        }
                    }
                    None => {}
                }
            }
            Some(sdl2::SDL_EventType::SDL_KEYUP) => {
                match FromPrimitive::from_i32(unsafe { event.key.keysym.sym }) {
                    Some(k) => {
                        let b = self.get_key_mut(k);
                        b.status = Status::Up as u8;
                    }
                    None => {}
                }
            }
            Some(sdl2::SDL_EventType::SDL_TEXTEDITING) => {}
            Some(sdl2::SDL_EventType::SDL_TEXTINPUT) => {
                let text =
                    unsafe { std::ffi::CStr::from_ptr(event.text.text.as_ptr() as *const _) }
                        .to_string_lossy()
                        .to_owned()
                        .to_string();
                self.input_text.push_str(&text);
            }
            _ => {}
        }
    }

    fn process_text_input_key(&mut self, key: sdl2::SDL_KeyCode) {
        match key {
            sdl2::SDL_KeyCode::SDLK_BACKSPACE => {
                if self.input_text.is_empty() {
                    self.input_backspace += 1;
                } else {
                    self.input_text.pop();
                }
            }
            sdl2::SDL_KeyCode::SDLK_DELETE => {
                self.input_delete += 1;
            }
            sdl2::SDL_KeyCode::SDLK_LEFT => {
                self.input_move -= 1;
            }
            sdl2::SDL_KeyCode::SDLK_RIGHT => {
                self.input_move += 1;
            }
            sdl2::SDL_KeyCode::SDLK_HOME => {
                self.input_seek = InputSeek::Start;
            }
            sdl2::SDL_KeyCode::SDLK_END => {
                self.input_seek = InputSeek::End;
            }
            _ => {}
        }
    }

    pub fn mouse_moved(&self) -> bool {
        self.mouse_delta.x != 0 || self.mouse_delta.y != 0
    }

    fn get_key_mut(&mut self, key: sdl2::SDL_KeyCode) -> &mut KeyButton {
        self.key_buttons
            .entry(key)
            .or_insert_with(|| KeyButton::new(key))
    }

    pub fn get_key(&self, key: sdl2::SDL_KeyCode) -> Option<&KeyButton> {
        match self.key_buttons.get(&key) {
            Some(kb) => Some(kb),
            None => None,
        }
    }

    pub fn get_sdl_mouse(&self, sdl_button_type: u8) -> &MouseButton {
        &self.mouse_buttons[sdl_button_type as usize]
    }

    pub fn get_mouse(&self, button: Mouse) -> &MouseButton {
        &self.mouse_buttons[button as usize]
    }
}
