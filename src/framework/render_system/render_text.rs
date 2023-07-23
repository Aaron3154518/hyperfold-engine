use shared::traits::{Call, SplitAround};

use crate::{
    components,
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
    render_data::{Fit, RenderAsset, RenderDataTrait, RenderTexture},
    shapes::{Rectangle, ShapeTrait},
    text::{parse_text, render_text, TextToken},
    AssetManager, Camera, RenderComponent, Screen,
};

pub enum TextImage {
    Render(RenderComponent),
    Reference(Entity),
}

#[macros::component]
pub struct RenderText {
    font_data: FontData,
    tokens: Vec<TextToken>,
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
            tokens: Vec::new(),
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

    pub fn with_text(mut self, text: &str) -> Self {
        self.set_text(text);
        self
    }

    pub fn set_text(&mut self, text: &str) {
        let tokens = parse_text(&text);
        if tokens != self.tokens {
            self.tokens = tokens;
            self.clear_texture();
        }
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

components!(
    RenderTextArgs,
    tex: &'a mut RenderComponent,
    pos: Option<&'a Position>
);

#[macros::system]
fn update_render_text(
    _ev: &PreRender,
    mut rcs: Vec<RenderTextArgs>,
    r: &super::Renderer,
    am: &mut AssetManager,
    screen: &Screen,
    camera: &Camera,
) {
    let n = rcs.len();
    for i in 0..n {
        let (left, RenderTextArgs { tex, pos, .. }, right) = rcs.split_around_mut(i);
        let pos = match pos {
            Some(pos) => pos,
            None => continue,
        };
        tex.try_mut(|rt: &mut RenderText| {
            let max_w = match rt.tex.get_render_data().dest.fit {
                Fit::Fit(false, _) => None,
                _ => Some(pos.0.w_u32()),
            };
            // Render text if no existing texture
            let tex = rt.tex.get_or_insert_texture(|| {
                render_text(
                    r,
                    am,
                    &rt.tokens,
                    &rt.font_data,
                    max_w,
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
                                let RenderTextArgs { eid: id, tex, .. } = if j > i {
                                    &mut right[j - i - 1]
                                } else {
                                    &mut left[j]
                                };
                                if *id == eid {
                                    draw(*tex, rect);
                                    break;
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
