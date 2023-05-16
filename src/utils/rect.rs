use crate::sdl2;

#[derive(Clone, Copy, Debug)]
pub struct Dimensions<T>
where
    T: Clone + Copy,
{
    pub w: T,
    pub h: T,
}

impl Dimensions<i32> {
    pub fn new() -> Self {
        Self { w: 0, h: 0 }
    }
}

impl Dimensions<u32> {
    pub fn new() -> Self {
        Self { w: 0, h: 0 }
    }
}

impl Dimensions<f32> {
    pub fn new() -> Self {
        Self { w: 0.0, h: 0.0 }
    }
}

pub type Point = sdl2::SDL_Point;
pub type PointF = sdl2::SDL_FPoint;

impl Point {
    pub fn new() -> Self {
        Self { x: 0, y: 0 }
    }

    pub fn dist(&self, p: &Point) -> f32 {
        (((self.x - p.x).pow(2) + (self.y - p.y).pow(2)) as f32).sqrt()
    }
}

impl PointF {
    pub fn new() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    pub fn dist(&self, p: &PointF) -> f32 {
        (((self.x - p.x).powf(2.0) + (self.y - p.y).powf(2.0)) as f32).sqrt()
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum Align {
    TopLeft = 0,
    Center,
    BotRight,
}

#[derive(Copy, Clone, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        }
    }

    // Don't force normalize (e.g. intersect needs negative widths)
    pub fn from(x: f32, y: f32, w: f32, h: f32, ax: Align, ay: Align) -> Self {
        let mut r = Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
        };
        r.set_x(x, ax);
        r.set_y(y, ay);
        r
    }

    pub fn from_corners(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        let (x, w) = if x1 < x2 {
            (x1, x2 - x1)
        } else {
            (x2, x1 - x2)
        };
        let (y, h) = if y1 < y2 {
            (y1, y2 - y1)
        } else {
            (y2, y1 - y2)
        };
        Rect { x, y, w, h }
    }

    pub fn from_sdl_rect(r: sdl2::SDL_Rect) -> Self {
        Rect {
            x: r.x as f32,
            y: r.y as f32,
            w: r.w as f32,
            h: r.h as f32,
        }
    }

    pub fn to_sdl_rect(&self) -> sdl2::SDL_Rect {
        sdl2::SDL_Rect {
            x: self.x_i32(),
            y: self.y_i32(),
            w: self.w_i32(),
            h: self.h_i32(),
        }
    }

    // getters - float x
    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn x2(&self) -> f32 {
        self.x + self.w
    }

    pub fn cx(&self) -> f32 {
        self.x + self.w / 2.0
    }

    pub fn get_x(&self, a: Align) -> f32 {
        match a {
            Align::TopLeft => self.x(),
            Align::Center => self.cx(),
            Align::BotRight => self.x2(),
        }
    }

    // float y
    pub fn y(&self) -> f32 {
        self.y
    }

    pub fn y2(&self) -> f32 {
        self.y + self.h
    }

    pub fn cy(&self) -> f32 {
        self.y + self.h / 2.0
    }

    pub fn get_y(&self, a: Align) -> f32 {
        match a {
            Align::TopLeft => self.y(),
            Align::Center => self.cy(),
            Align::BotRight => self.y2(),
        }
    }

    // x and y
    pub fn get_pos(&self, ax: Align, ay: Align) -> PointF {
        PointF {
            x: self.get_x(ax),
            y: self.get_y(ay),
        }
    }

    pub fn center(&self) -> PointF {
        self.get_pos(Align::Center, Align::Center)
    }

    // float w/h
    pub fn w(&self) -> f32 {
        self.w
    }

    pub fn h(&self) -> f32 {
        self.h
    }

    pub fn dim(&self) -> Dimensions<f32> {
        Dimensions {
            w: self.w,
            h: self.h,
        }
    }

    pub fn half_w(&self) -> f32 {
        self.w / 2.0
    }

    pub fn half_h(&self) -> f32 {
        self.h / 2.0
    }

    pub fn min_dim(&self) -> f32 {
        self.w.min(self.h)
    }

    // int x
    pub fn x_i32(&self) -> i32 {
        self.x().round() as i32
    }

    pub fn x2_i32(&self) -> i32 {
        self.x2().round() as i32
    }

    pub fn cx_i32(&self) -> i32 {
        self.cx().round() as i32
    }

    pub fn get_x_i32(&self, a: Align) -> i32 {
        self.get_x(a).round() as i32
    }

    // int y
    pub fn y_i32(&self) -> i32 {
        self.y().round() as i32
    }

    pub fn y2_i32(&self) -> i32 {
        self.y2().round() as i32
    }

    pub fn cy_i32(&self) -> i32 {
        self.cy().round() as i32
    }

    pub fn get_y_i32(&self, a: Align) -> i32 {
        self.get_y(a).round() as i32
    }

    // x and y
    pub fn get_pos_i32(&self, ax: Align, ay: Align) -> Point {
        Point {
            x: self.get_x_i32(ax),
            y: self.get_y_i32(ay),
        }
    }

    pub fn center_i32(&self) -> Point {
        self.get_pos_i32(Align::Center, Align::Center)
    }

    // int dimensions
    pub fn w_i32(&self) -> i32 {
        self.w.round() as i32
    }

    pub fn h_i32(&self) -> i32 {
        self.h().round() as i32
    }

    pub fn dim_i32(&self) -> Dimensions<i32> {
        Dimensions {
            w: self.w_i32(),
            h: self.h_i32(),
        }
    }

    pub fn half_w_i32(&self) -> i32 {
        self.half_w().round() as i32
    }

    pub fn half_h_i32(&self) -> i32 {
        self.half_h().round() as i32
    }

    pub fn min_dim_i32(&self) -> i32 {
        self.min_dim().round() as i32
    }

    // setters
    pub fn set_x(&mut self, val: f32, a: Align) {
        match a {
            Align::TopLeft => self.x = val,
            Align::Center => self.x = val - self.w / 2.0,
            Align::BotRight => self.x = val - self.w,
        }
    }

    pub fn set_y(&mut self, val: f32, a: Align) {
        match a {
            Align::TopLeft => self.y = val,
            Align::Center => self.y = val - self.h / 2.0,
            Align::BotRight => self.y = val - self.h,
        }
    }

    pub fn set_pos(&mut self, x: f32, y: f32, ax: Align, ay: Align) {
        self.set_x(x, ax);
        self.set_y(y, ay);
    }

    pub fn set_w(&mut self, w: f32, a: Align) {
        match a {
            Align::TopLeft => {
                self.w = w;
            }
            Align::Center => {
                self.x += (self.w - w) / 2.0;
                self.w = w;
            }
            Align::BotRight => {
                self.x += self.w - w;
                self.w = w;
            }
        }
        self.normalize();
    }

    pub fn set_h(&mut self, h: f32, a: Align) {
        match a {
            Align::TopLeft => {
                self.h = h;
            }
            Align::Center => {
                self.y += (self.h - h) / 2.0;
                self.h = h;
            }
            Align::BotRight => {
                self.y += self.h - h;
                self.h = h;
            }
        }
        self.normalize();
    }

    pub fn set_dim(&mut self, w: f32, h: f32, ax: Align, ay: Align) {
        self.set_w(w, ax);
        self.set_h(h, ay);
    }

    // other
    pub fn empty(&self) -> bool {
        self.w == 0.0 || self.h == 0.0
    }

    pub fn invalid(&self) -> bool {
        self.w < 0.0 || self.h < 0.0
    }

    pub fn normalize(&mut self) {
        if self.w < 0.0 {
            self.x += self.w;
            self.w = -self.w;
        }
        if self.h < 0.0 {
            self.y += self.h;
            self.h = -self.h;
        }
    }

    pub fn move_by(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy;
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
    }

    pub fn move_by_factor(&mut self, x_factor: f32, y_factor: f32, ax: Align, ay: Align) {
        self.set_x(self.get_x(ax) * x_factor, ax);
        self.set_y(self.get_y(ay) * y_factor, ay);
    }

    pub fn resize(&mut self, factor: f32, ax: Align, ay: Align) {
        self.set_dim(self.w * factor, self.h * factor, ax, ay)
    }

    pub fn expand(&mut self, dw: f32, dh: f32, ax: Align, ay: Align) {
        self.set_dim(self.w + dw, self.h + dh, ax, ay)
    }

    pub fn fit_within(&mut self, r: &Rect) {
        self.set_x(self.x.min(r.x2() - self.w).max(r.x()), Align::TopLeft);
        self.set_y(self.y.min(r.y2() - self.h).max(r.y()), Align::TopLeft);
    }

    pub fn intersects(&self, r: &Rect) -> bool {
        (self.x <= r.x2() && self.x2() >= r.x) && (self.y <= r.y2() && self.y2() >= r.y)
    }

    pub fn intersect(&self, r: &Rect) -> Option<Rect> {
        let i = Rect::from_corners(
            self.x.max(r.x),
            self.y.max(r.y),
            self.x2().min(r.x2()),
            self.y2().min(r.y2()),
        );
        (i.w > 0.0 && i.h > 0.0).then_some(i)
    }

    pub fn get_min_rect(w: f32, h: f32, max_w: f32, max_h: f32) -> Rect {
        let w_ratio = w / max_w;
        let h_ratio = h / max_h;
        let ratio = w_ratio.min(h_ratio);
        let w = w / ratio;
        let h = h / ratio;
        let x = (max_w - w) / 2.0;
        let y = (max_h - h) / 2.0;
        Rect::from(x, y, w, h, Align::TopLeft, Align::TopLeft)
    }
}

impl std::fmt::Display for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({:.2}, {:.2}) -> ({:.2}, {:.2}), ({:.2}, {:.2}), {:.2}x{:.2}",
            self.x(),
            self.y(),
            self.x2(),
            self.y2(),
            self.cx(),
            self.cy(),
            self.w(),
            self.h()
        )
    }
}
