use shared::util::{Call, SplitAround};

use crate::{
    ecs::{entities::Entity, events::core::PreRender},
    framework::physics::Position,
    sdl2,
    utils::{
        colors::{BLACK, GRAY},
        rect::{Align, Rect},
        util::{AsType, TryAsType},
    },
};

use super::{
    drawable::{Canvas, Drawable},
    font::FontData,
    rect_to_camera_coords,
    render_data::{RenderAsset, RenderDataTrait, RenderTexture},
    shapes::{Rectangle, ShapeTrait},
    text::render_text,
    AssetManager, Camera, RenderComponent, Screen,
};

pub enum TextImage {
    Render(RenderComponent),
    Reference(Entity),
}

#[macros::component]
pub struct RenderText {
    font_data: FontData,
    text: String,
    imgs: Vec<TextImage>,
    img_rects: Vec<Rect>,
    tex: RenderTexture,
    color: sdl2::SDL_Color,
    bkgrnd: sdl2::SDL_Color,
    align_x: Align,
    align_y: Align,
}

impl RenderText {
    pub fn new(font_data: FontData) -> Self {
        Self {
            font_data,
            text: String::new(),
            imgs: Vec::new(),
            img_rects: Vec::new(),
            tex: RenderTexture::new(None),
            color: BLACK,
            bkgrnd: GRAY,
            align_x: Align::Center,
            align_y: Align::Center,
        }
    }

    fn clear_texture(&mut self) {
        self.tex.set_texture(None);
    }

    pub fn set_font_data(&mut self, font_data: FontData) {
        self.font_data = font_data;
        self.clear_texture();
    }

    pub fn with_text(mut self, text: String) -> Self {
        self.set_text(text);
        self
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.clear_texture();
    }

    pub fn with_text_color(mut self, color: sdl2::SDL_Color) -> Self {
        self.set_text_color(color);
        self
    }

    pub fn set_text_color(&mut self, color: sdl2::SDL_Color) {
        self.color = color;
        self.clear_texture();
    }

    pub fn with_background_color(mut self, bkgrnd: sdl2::SDL_Color) -> Self {
        self.set_background_color(bkgrnd);
        self
    }

    pub fn set_background_color(&mut self, bkgrnd: sdl2::SDL_Color) {
        self.bkgrnd = bkgrnd;
        self.clear_texture();
    }

    pub fn with_text_align(mut self, ax: Align, ay: Align) -> Self {
        self.set_text_align(ax, ay);
        self
    }

    pub fn set_text_align(&mut self, ax: Align, ay: Align) {
        (self.align_x, self.align_y) = (ax, ay);
        self.clear_texture();
    }

    pub fn with_images(mut self, imgs: Vec<TextImage>) -> Self {
        self.set_images(imgs);
        self
    }

    pub fn set_images(&mut self, imgs: Vec<TextImage>) {
        self.imgs = imgs;
    }
}

impl RenderDataTrait for RenderText {
    fn get_render_data<'a>(&'a self) -> &'a super::render_data::RenderData {
        self.tex.get_render_data()
    }

    fn get_render_data_mut<'a>(&'a mut self) -> &'a mut super::render_data::RenderData {
        self.tex.get_render_data_mut()
    }
}

impl Drawable for RenderText {
    fn draw(&mut self, r: &super::Renderer) {
        self.tex.draw(r)
    }
}

#[macros::system]
fn update_render_text(
    _ev: &PreRender,
    mut rcs: Vec<(&Entity, &Position, &mut RenderComponent)>,
    r: &super::Renderer,
    am: &mut AssetManager,
    screen: &Screen,
    camera: &Camera,
) {
    let n = rcs.len();
    for i in 0..n {
        let (left, (.., pos, rc), right) = rcs.split_around_mut(i);
        rc.try_mut(|rt: &mut RenderText| {
            // Render text if no existing texture
            let tex = rt.tex.get_or_insert_texture(|| {
                render_text(
                    r,
                    am,
                    &rt.text,
                    &rt.font_data,
                    pos.0,
                    rt.color,
                    rt.bkgrnd,
                    rt.align_x,
                    rt.align_y,
                )
                .call_into(|(t, rects)| {
                    rt.img_rects = rects;
                    t
                })
            });

            // TODO: only if needed
            // TODO: don't require position component -> Optional components
            // Redraw images
            let mut draw = |rc: &mut RenderComponent, rect: Rect| {
                rc.try_mut(|rt: &mut RenderTexture| rt.get_render_data_mut().set_dest_rect(rect))
                    .try_mut(rc, |ra: &mut RenderAsset| {
                        ra.get_render_data_mut().set_dest_rect(rect)
                    });
                tex.draw(r, &mut Rectangle::new().set_color(rt.bkgrnd).fill(rect));
                tex.draw_asset(r, am, rc);
            };

            for (&rect, img) in rt.img_rects.iter().zip(rt.imgs.iter_mut()) {
                match img {
                    TextImage::Reference(eid) => {
                        for j in 0..n {
                            if i != j {
                                let (id, .., rc) = if j > i {
                                    &mut right[j - i - 1]
                                } else {
                                    &mut left[j]
                                };
                                if *id == eid {
                                    draw(*rc, rect);
                                }
                            }
                        }
                    }
                    TextImage::Render(rc) => draw(rc, rect),
                }
            }

            rt.tex
                .set_dest_rect(rect_to_camera_coords(&pos.0, screen, camera));
        });
    }
}
