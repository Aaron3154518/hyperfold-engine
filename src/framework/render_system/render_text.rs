use shared::util::Call;

use crate::{
    ecs::{entities::Entity, events::core::PreRender},
    framework::physics::Position,
    utils::{
        rect::{Align, Rect},
        util::{AsType, TryAsType},
    },
};

use super::{
    drawable::{Canvas, Drawable},
    font::FontData,
    rect_to_camera_coords,
    render_data::{RenderAsset, RenderDataTrait, RenderTexture},
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
            align_x: Align::Center,
            align_y: Align::Center,
        }
    }

    pub fn set_font_data(&mut self, font_data: FontData) {
        self.font_data = font_data;
        self.tex.set_texture(None);
    }

    pub fn with_text(mut self, text: String) -> Self {
        self.set_text(text);
        self
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.tex.set_texture(None);
    }

    pub fn with_text_align(mut self, ax: Align, ay: Align) -> Self {
        self.set_text_align(ax, ay);
        self
    }

    pub fn set_text_align(&mut self, ax: Align, ay: Align) {
        (self.align_x, self.align_y) = (ax, ay);
        self.tex.set_texture(None);
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
    rc: &mut RenderComponent,
    pos: &Position,
    r: &super::Renderer,
    am: &mut AssetManager,
    screen: &Screen,
    camera: &Camera,
) {
    rc.try_mut(|rt: &mut RenderText| {
        // Render text if no existing texture
        let tex = rt.tex.get_or_insert_texture(|| {
            render_text(
                r,
                am,
                &rt.text,
                &rt.font_data,
                pos.0,
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
        for (&rect, img) in rt.img_rects.iter().zip(rt.imgs.iter_mut()) {
            // if let Some(rc) = match img {
            //     TextImage::Render(rc) => Some(rc),
            //     TextImage::Reference(id) => am.get_render_by_id_mut(*id),
            // } {
            //     rc.set_dest_rect(rect);
            //     tex.draw_asset(r, am, rc);
            // }
            if let TextImage::Render(rc) = img {
                rc.try_mut(|rt: &mut RenderTexture| rt.get_render_data_mut().set_dest_rect(rect))
                    .try_mut(rc, |ra: &mut RenderAsset| {
                        ra.get_render_data_mut().set_dest_rect(rect)
                    });
                tex.draw_asset(r, am, rc);
            }
        }

        rt.tex
            .set_dest_rect(rect_to_camera_coords(&pos.0, screen, camera));
    });
}
