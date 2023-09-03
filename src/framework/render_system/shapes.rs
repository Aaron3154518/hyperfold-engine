use std::f32::consts::{PI, TAU};

use num_traits::Pow;
use shared::traits::Call;

use crate::{
    impl_as_any, sdl2,
    utils::{
        colors::BLACK,
        rect::{Align, Point, PointF, Rect},
        util::{normalize_rad, CrossProduct, FloatMath, IntMath, DEG_TO_RAD, F32_ERR, HALF_PI},
    },
};

use super::{drawable::Drawable, Renderer};

// Shape data
pub trait ShapeTrait {
    fn shape_data<'a>(&'a self) -> &'a ShapeData;

    fn shape_data_mut<'a>(&'a mut self) -> &'a mut ShapeData;

    fn set_shape_data(mut self, data: ShapeData) -> Self
    where
        Self: Sized,
    {
        *self.shape_data_mut() = data;
        self
    }

    fn set_color(mut self, color: sdl2::SDL_Color) -> Self
    where
        Self: Sized,
    {
        self.shape_data_mut().color = color;
        self
    }

    fn set_blendmode(mut self, mode: sdl2::SDL_BlendMode) -> Self
    where
        Self: Sized,
    {
        self.shape_data_mut().blendmode = mode;
        self
    }

    fn set_boundary(mut self, bounds: Rect) -> Self
    where
        Self: Sized,
    {
        self.shape_data_mut().boundary = Some(bounds);
        self
    }

    fn get_boundary(&self, r: &Renderer) -> Option<Rect> {
        let size = r.output_size();
        let screen_bounds = Rect {
            x: 0.0,
            y: 0.0,
            w: size.w as f32,
            h: size.h as f32,
        };

        let shape_bounds = &self.shape_data().boundary;
        match shape_bounds {
            Some(sb) => screen_bounds.intersect(sb),
            None => Some(screen_bounds),
        }
    }

    fn set_draw_state(&self, r: &Renderer) {
        let ShapeData {
            color, blendmode, ..
        } = self.shape_data();
        r.set_color(*color);
        r.set_blendmode(*blendmode);
    }
}

#[derive(Clone, Copy)]
pub struct ShapeData {
    color: sdl2::SDL_Color,
    blendmode: sdl2::SDL_BlendMode,
    boundary: Option<Rect>,
}

impl ShapeData {
    pub fn new() -> Self {
        Self {
            color: BLACK,
            blendmode: sdl2::SDL_BlendMode::SDL_BLENDMODE_NONE,
            boundary: None,
        }
    }
}

impl ShapeTrait for ShapeData {
    fn shape_data<'a>(&'a self) -> &'a ShapeData {
        self
    }

    fn shape_data_mut<'a>(&'a mut self) -> &'a mut ShapeData {
        self
    }
}

// Rectangle
#[derive(Clone, Copy)]
pub struct RectangleData {
    r1: Option<Rect>,
    r2: Option<Rect>,
}

#[derive(Clone, Copy)]
pub struct Rectangle {
    data: RectangleData,
    shape: ShapeData,
}

impl Rectangle {
    pub fn new() -> Self {
        Self {
            data: RectangleData { r1: None, r2: None },
            shape: ShapeData::new(),
        }
    }

    pub fn fill(mut self, r: Rect) -> Self {
        self.data.r2 = Some(r);
        self
    }

    pub fn except(mut self, r: Rect) -> Self {
        self.data.r1 = Some(r);
        self
    }

    pub fn border(mut self, r: Rect, mut thickness: f32, center: bool) -> Self {
        let (mut r1, mut r2) = (r, r);
        if center {
            thickness = thickness.abs();
            r1.set_dim(
                r1.w() - thickness,
                r1.h() - thickness,
                Align::Center,
                Align::Center,
            );
            r2.set_dim(
                r2.w() + thickness,
                r2.h() + thickness,
                Align::Center,
                Align::Center,
            );
        } else {
            let dw = 2.0 * thickness;
            if dw > 0.0 { &mut r2 } else { &mut r1 }.expand(dw, dw, Align::Center, Align::Center);
        }
        self.data.r1 = Some(r1);
        self.data.r2 = Some(r2);
        self
    }

    pub fn data(&self) -> RectangleData {
        self.data
    }
}

impl ShapeTrait for Rectangle {
    fn shape_data<'a>(&'a self) -> &'a ShapeData {
        &self.shape
    }

    fn shape_data_mut<'a>(&'a mut self) -> &'a mut ShapeData {
        &mut self.shape
    }
}

impl Drawable for Rectangle {
    fn draw(&mut self, r: &Renderer) {
        self.set_draw_state(r);

        let fill_r = match self.get_boundary(r) {
            Some(r) => r,
            None => return,
        };

        let r2 = match self.data.r2 {
            Some(r) => r,
            None => {
                // Fill entire boundary
                r.fill_rect(fill_r);
                return;
            }
        };

        // Intersect r2 with bounds
        let fill_r = match fill_r.intersect(&r2) {
            Some(r) => r,
            None => return,
        };

        // Intersect r1 with new bounds
        let r1 = match self.data.r1.and_then(|r1| fill_r.intersect(&r1)) {
            Some(r) => r,
            None => {
                // Fill r2
                r.fill_rect(fill_r);
                return;
            }
        };

        // Fill bounds except for r
        // Left, right, top, bottom
        for side in [
            Rect {
                x: r2.x(),
                y: r2.y(),
                w: r1.x() - r2.x(),
                h: r2.y2() - r2.y(),
            },
            Rect {
                x: r1.x2(),
                y: r2.y(),
                w: r2.x2() - r1.x2(),
                h: r2.y2() - r2.y(),
            },
            Rect {
                x: r2.x(),
                y: r2.y(),
                w: r2.x2() - r2.x(),
                h: r1.y() - r2.y(),
            },
            Rect {
                x: r2.x(),
                y: r1.y2(),
                w: r2.x2() - r2.x(),
                h: r2.y2() - r1.y2(),
            },
        ] {
            if !side.empty() && !side.invalid() {
                Rectangle::new()
                    .set_shape_data(self.shape)
                    .fill(side)
                    .draw(r);
            }
        }
    }
}

impl_as_any!(Rectangle);

// Circle
#[derive(Clone, Copy)]
pub struct CircleData {
    c: Point,
    r1: u32,
    r2: u32,
    a1_rad: f32,
    a2_rad: f32,
    dashes: u16,
}

#[derive(Clone, Copy)]
enum Quadrant {
    UpperRight,
    UpperLeft,
    BottomLeft,
    BottomRight,
}

impl Quadrant {
    pub fn next(&self) -> Self {
        match self {
            Quadrant::UpperRight => Quadrant::UpperLeft,
            Quadrant::UpperLeft => Quadrant::BottomLeft,
            Quadrant::BottomLeft => Quadrant::BottomRight,
            Quadrant::BottomRight => Quadrant::UpperRight,
        }
    }
}

impl From<f32> for Quadrant {
    fn from(rad: f32) -> Self {
        match (normalize_rad(rad) / HALF_PI).floor_i32() {
            0 => Quadrant::UpperRight,
            1 => Quadrant::UpperLeft,
            2 => Quadrant::BottomLeft,
            3 => Quadrant::BottomRight,
            _ => panic!("Could not convert angle to quadrant: {rad}"),
        }
    }
}

#[derive(Clone, Copy)]
struct Sector {
    a1_rad: f32,
    a2_rad: f32,
    quad: Quadrant,
}

#[derive(Clone, Copy)]
pub struct Circle {
    data: CircleData,
    shape_data: ShapeData,
}

impl Circle {
    pub fn new() -> Self {
        Self {
            data: CircleData {
                c: Point::new(),
                r1: 0,
                r2: 0,
                a1_rad: 0.0,
                a2_rad: TAU,
                dashes: 0,
            },
            shape_data: ShapeData::new(),
        }
    }

    pub fn set_center(mut self, c: Point) -> Self {
        self.data.c = c;
        self
    }

    pub fn fill(mut self, r: u32) -> Self {
        self.data.r2 = r;
        self
    }

    pub fn except(mut self, r: u32) -> Self {
        self.data.r1 = r;
        self
    }

    pub fn border(mut self, r: u32, thickness: i32, center: bool) -> Self {
        let abs_thick = thickness.unsigned_abs();
        if center {
            self.data.r2 = r + abs_thick / 2;
        } else {
            self.data.r2 = if thickness < 0 { r } else { r + abs_thick };
        }
        self.data.r1 = 0.max(self.data.r2 - abs_thick);
        self
    }

    pub fn full_circle(mut self) -> Self {
        (self.data.a1_rad, self.data.a2_rad) = (0.0, TAU);
        self
    }

    pub fn set_angle_rad(mut self, a1: f32, a2: f32) -> Self {
        (self.data.a1_rad, self.data.a2_rad) = (normalize_rad(a1), normalize_rad(a2));
        self
    }

    pub fn set_angle_deg(self, a1: f32, a2: f32) -> Self {
        self.set_angle_rad(a1 * DEG_TO_RAD, a2 * DEG_TO_RAD)
    }

    pub fn dashed(mut self, dashes: u16) -> Self {
        self.data.dashes = dashes;
        self
    }

    pub fn data(&self) -> CircleData {
        self.data
    }

    fn draw_circle(&self, bounds: Rect, r: &Renderer) {
        // Circle
        let mut dx = 0;
        while dx < self.data.r2 {
            let dy1 = if dx >= self.data.r1 {
                0
            } else {
                (self.data.r1.pow(2) - dx.pow(2)).sqrt().round_i32()
            };
            let dy2 = (self.data.r2.pow(2) - dx.pow(2)).sqrt().round_i32();
            // Iterate through dx, -dx
            for dx in [dx as i32, -(dx as i32)] {
                let x = self.data.c.x + dx;
                // Make sure x is in bounds
                if x >= bounds.x_i32() && x <= bounds.x2_i32() {
                    // Iterate through [dy1, dy2], [-dy2, -dy1]
                    for (dy1, dy2) in [(dy1, dy2), (-dy2, -dy1)] {
                        let y1 = bounds.y_i32().max(self.data.c.y + dy1);
                        let y2 = bounds.y2_i32().min(self.data.c.y + dy2);
                        // Make sure at least one y is in bounds
                        if y1 <= bounds.y2_i32() && y2 >= bounds.y_i32() {
                            r.draw_line(x, y1, x, y2)
                        }
                    }
                }
            }
            dx += 1;
        }
    }

    fn draw_sectors(&self, bounds: Rect, r: &Renderer) {
        let mut da = self.data.a2_rad - self.data.a1_rad;
        if da < 0.0 {
            da += TAU;
        }

        // Draw sectors
        let mut s_a2 = self.data.a1_rad;
        let mut s_quad = Quadrant::from(self.data.a1_rad);
        while da > 0.0 {
            let s_a1 = s_a2 % HALF_PI;
            let s_da = da.min(HALF_PI).min(HALF_PI - s_a1);
            s_a2 = s_a1 + s_da;
            self.draw_sector(
                Sector {
                    a1_rad: s_a1,
                    a2_rad: s_a2,
                    quad: s_quad,
                },
                bounds,
                r,
            );
            s_quad = s_quad.next();
            da -= s_da;
        }
    }

    fn draw_sector(&self, s: Sector, bounds: Rect, r: &Renderer) {
        let flip = match s.quad {
            Quadrant::UpperRight | Quadrant::UpperLeft => false,
            Quadrant::BottomLeft | Quadrant::BottomRight => true,
        };

        let (sin_a1, cos_a1) = s.a1_rad.sin_cos();
        let (sin_a2, cos_a2) = s.a2_rad.sin_cos();

        let [p11, p12, p21, p22] = [self.data.r1 as f32, self.data.r2 as f32]
            .cross(&[(cos_a1, sin_a1), (cos_a2, sin_a2)])
            .map(|(r, (cos, sin))| PointF {
                x: r * cos,
                y: r * sin,
            });

        let [v1, v2] = [(p21, p11), (p22, p12)].map(|(p2, p1)| PointF {
            x: p2.x - p1.x,
            y: p2.y - p1.y,
        });

        let (m1_inf, m2_inf) = (v1.x.abs() < F32_ERR, v2.x.abs() < F32_ERR);
        let [m1, m2] =
            [(m1_inf, v1), (m2_inf, v2)].map(|(inf, v)| if inf { 0.0 } else { v.y / v.x });
        let [b1, b2] = [(p11, m1), (p22, m2)].map(|(p, m)| p.y - m * p.x);

        let mut off_x = p12.x.ceil();
        while off_x <= p21.x {
            let mut dy1 = if off_x < p11.x {
                (self.data.r1.pow(2) as f32 - off_x.pow(2.0)).sqrt()
            } else {
                if m1_inf {
                    self.data.r1 as f32
                } else {
                    m1 * off_x + b1
                }
            }
            .round_i32();

            let mut dy2 = if off_x < p22.x {
                if m2_inf {
                    self.data.r2 as f32
                } else {
                    m2 * off_x + b2
                }
            } else {
                (self.data.r2.pow(2) as f32 - off_x.pow(2.0)).sqrt()
            }
            .round_i32();

            let mut dx = off_x.round_i32();
            if flip {
                (dx, dy1, dy2) = (-dx, -dy1, -dy2);
            }

            match s.quad {
                Quadrant::UpperRight | Quadrant::BottomLeft => (dx, -dy1, dx, -dy2),
                Quadrant::UpperLeft | Quadrant::BottomRight => (-dy1, -dx, -dy2, -dx),
            }
            .call_into(|(dx1, dy1, dx2, dy2)| {
                let (x1, y1, x2, y2) = (
                    self.data.c.x + dx1,
                    self.data.c.y + dy1,
                    self.data.c.x + dx2,
                    self.data.c.y + dy2,
                );

                if x1 >= bounds.x2_i32()
                    || y1 >= bounds.y2_i32()
                    || x2 <= bounds.x_i32()
                    || y2 <= bounds.y_i32()
                {
                    return;
                }

                r.draw_line(
                    x1.max(bounds.x_i32()),
                    y1.max(bounds.y_i32()),
                    x2.min(bounds.x2_i32()),
                    y2.min(bounds.y2_i32()),
                )
            });

            off_x += 1.0;
        }
    }

    fn draw_dashed(&self, bounds: Rect, r: &Renderer) {
        let da = PI / self.data.dashes as f32;
        let max_a = if self.data.a1_rad <= self.data.a2_rad {
            self.data.a2_rad
        } else {
            self.data.a2_rad + TAU
        };
        let circle = *self;
        let mut s_a1 = self.data.a1_rad;
        while s_a1 < max_a {
            circle
                .set_angle_rad(s_a1, max_a.min(s_a1 + da))
                .draw_sectors(bounds, r);
            s_a1 += da * 2.0;
        }
    }
}

impl ShapeTrait for Circle {
    fn shape_data<'a>(&'a self) -> &'a ShapeData {
        &self.shape_data
    }

    fn shape_data_mut<'a>(&'a mut self) -> &'a mut ShapeData {
        &mut self.shape_data
    }
}

impl Drawable for Circle {
    fn draw(&mut self, r: &Renderer) {
        self.set_draw_state(r);

        let bounds = match self.get_boundary(r) {
            Some(r) => r,
            None => return,
        };

        match self.data.dashes {
            0 => {
                let mut da = self.data.a2_rad - self.data.a1_rad;
                if da < 0.0 {
                    da += TAU
                }
                if (da - TAU).abs() < F32_ERR {
                    self.draw_circle(bounds, r)
                } else {
                    self.draw_sectors(bounds, r)
                }
            }
            _ => {
                self.draw_dashed(bounds, r);
                return;
            }
        }
    }
}

impl_as_any!(Circle);

// Brighten
#[derive(Clone, Copy)]
pub struct Brighten {
    strength: u8,
}

impl Brighten {
    pub fn new(strength: u8) -> Self {
        Self { strength }
    }

    pub fn get_strength(&self) -> u8 {
        self.strength
    }
}

impl Drawable for Brighten {
    fn draw(&mut self, r: &Renderer) {
        Rectangle::new()
            .set_color(sdl2::SDL_Color {
                r: self.strength,
                g: self.strength,
                b: self.strength,
                a: 255,
            })
            .set_blendmode(sdl2::SDL_BlendMode::SDL_BLENDMODE_ADD)
            .draw(r)
    }
}

impl_as_any!(Brighten);
