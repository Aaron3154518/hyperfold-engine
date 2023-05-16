use crate::{
    sdl2,
    utils::{
        colors::BLACK,
        rect::{Align, Rect},
    },
};

use super::{
    renderer::RendererTrait,
    texture_builder::{Drawable, TextureBuilder},
};

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

    fn get_boundary(&self, r: &impl RendererTrait) -> Option<Rect> {
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
        self.data.r2 = (!r.empty() && !r.invalid()).then_some(r);
        self
    }

    pub fn except(mut self, r: Rect) -> Self {
        self.data.r1 = (!r.empty() && !r.invalid()).then_some(r);
        self
    }

    pub fn border(mut self, r: Rect, mut thickness: f32, center: bool) -> Self {
        if r.empty() || r.invalid() {
            (self.data.r1, self.data.r2) = (None, None);
            return self;
        }

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
        self.data.r1 = (!r1.invalid()).then_some(r1);
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
    fn draw(&self, tex: &TextureBuilder, r: &impl RendererTrait) {
        r.set_color(self.shape.color);
        r.set_blendmode(self.shape.blendmode);

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
                    .draw(tex, r);
            }
        }
    }
}

// // Circle
// struct Circle : public Drawable,
//                 public ShapeColor,
//                 public ShapeBlendmode,
//                 public ShapeBoundary {
//     struct Data {
//         SDL_Point c{0, 0};
//         int r1 = 0, r2 = 0;
//         float a1 = 0, a2 = TWO_PI;
//         unsigned int dashes = 0;
//     };

//     struct Sector {
//         float a1, a2;
//         int quad;
//     };

//     const Data &operator()() const;

//     void setCenter(SDL_Point c);
//     void setRadius(int r);
//     void setRadius(int r, int thickness, bool center = false);
//     void setFullCircle();
//     void setAngleRad(float a1, float a2);
//     void setAngleDeg(float a1, float a2);
//     void setDashed(unsigned int dashes);

//     void draw(TextureBuilder &tex);

//    private:
//     Data data;

//     void drawCircle(const Rect &bounds) const;
//     void drawSectors(const Rect &bounds) const;
//     void drawSector(const Sector &s, const Rect &bounds) const;
//     void drawDashed(const Rect &bounds) const;
// };
