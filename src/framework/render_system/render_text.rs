use shared::util::Call;

use crate::{
    ecs::entities::Entity,
    utils::rect::{Align, Rect},
};

use super::{
    drawable::AssetDrawable,
    font::FontData,
    render_data::{RenderData, RenderDataTrait, RenderTexture},
    text::render_text,
    AssetManager, RenderComponent,
};

pub enum TextImage {
    Render(RenderComponent),
    Reference(Entity),
}

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
    fn get_render_data<'a>(&'a self) -> &'a RenderData {
        self.tex.get_render_data()
    }

    fn get_render_data_mut<'a>(&'a mut self) -> &'a mut RenderData {
        self.tex.get_render_data_mut()
    }
}

impl AssetDrawable for RenderText {
    fn draw(&mut self, r: &super::Renderer, am: &mut AssetManager) {
        // Render text
        let dest = self.tex.get_render_data().dest.rect;
        let tex = self.tex.get_or_insert_texture(|| {
            render_text(
                r,
                am,
                &self.text,
                &self.font_data,
                dest,
                self.align_x,
                self.align_y,
            )
            .call_into(|(t, rects)| {
                self.img_rects = rects;
                t
            })
        });
        // Draw images
        for (&rect, img) in self.img_rects.iter().zip(self.imgs.iter_mut()) {
            // if let Some(rc) = match img {
            //     TextImage::Render(rc) => Some(rc),
            //     TextImage::Reference(id) => am.get_render_by_id_mut(*id),
            // } {
            //     rc.set_dest_rect(rect);
            //     tex.draw_asset(r, am, rc);
            // }
            if let TextImage::Render(rc) = img {
                rc.set_dest_rect(rect);
                tex.draw_asset(r, am, rc);
            }
        }
        // Draw final
        self.tex.draw(r, am);
    }
}
