use shared::util::{Call, FindFrom};

use crate::{
    sdl2,
    utils::rect::{Align, Dimensions, Rect},
};

use super::{
    drawable::Canvas,
    font::{Font, FontData},
    render_data::{FitMode, RectMode, RenderDataBuilderTrait, RenderTexture},
    AssetManager, Renderer, Texture,
};

// Text
#[derive(Debug)]
pub struct Text {
    start: usize,
    end: usize,
    w: u32,
}

impl Text {
    pub fn draw(
        &self,
        r: &Renderer,
        tex: &Texture,
        rect: Rect,
        font: &Font,
        text: &str,
        color: sdl2::SDL_Color,
    ) {
        let text_tex =
            r.create_texture_from_surface(font.render(&text[self.start..self.end], color));
        tex.draw(
            r,
            &mut RenderTexture::new(Some(text_tex)).with_dest(
                rect,
                RectMode::Absolute,
                FitMode::FitWithin(Align::Center, Align::Center),
            ),
        );
    }
}

// Line
#[derive(Debug)]
enum LineItem {
    Text(Text),
    Image,
}

#[derive(Debug)]
pub struct Line {
    w: u32,
    img_cnt: usize,
    items: Vec<LineItem>,
}

impl Line {
    pub fn new() -> Self {
        Self {
            w: 0,
            img_cnt: 0,
            items: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn space(&self, max_w: u32) -> u32 {
        max_w - self.w
    }

    pub fn add_text(&mut self, t: Text) {
        self.w += t.w;
        self.items.push(LineItem::Text(t));
    }

    pub fn add_image(&mut self, line_h: u32) {
        self.w += line_h;
        self.img_cnt += 1;
        self.items.push(LineItem::Image);
    }
}

// split text
fn add_text(
    lines: &mut Vec<Line>,
    font: &Font,
    text: &str,
    max_w: u32,
    space_w: u32,
    pos1: usize,
    pos2: usize,
) {
    if pos1 >= pos2 {
        return;
    }

    let last_line = match lines.last_mut() {
        Some(l) => l,
        None => return,
    };

    let segment = &text[pos1..pos2];
    let (width, count) = font
        .measure(segment, last_line.space(max_w))
        .call_into(|(w, c)| (w as u32, c as usize));

    if count == pos2 - pos1 {
        // Fit entire text onto line
        last_line.add_text(Text {
            start: pos1,
            end: pos2,
            w: width,
        });
    } else {
        // Break up text
        // Find last space, if any
        let new_start = match segment.rfind(' ') {
            // Break into words
            Some(last_space) => {
                let Dimensions { w: text_w, .. } = font.size_text(&text[..last_space]);
                last_line.add_text(Text {
                    start: pos1,
                    end: pos1 + last_space,
                    w: text_w as u32,
                });
                pos1 + last_space + 1
            }
            // Won't fit on this line
            None => {
                // Get the length until the next break
                let Dimensions { w: word_w, .. } =
                    font.size_text(segment.split_once(" ").map_or(segment, |(s, _)| s));
                if word_w as u32 <= max_w {
                    // It will fit on the next line
                    pos1
                } else {
                    // It is bigger than one line, split across multiple lines
                    last_line.add_text(Text {
                        start: pos1,
                        end: pos1 + count,
                        w: width,
                    });
                    pos1 + count + 1
                }
            }
        };
        lines.push(Line::new());
        add_text(lines, font, text, max_w, space_w, new_start, pos2);
    }
}

pub fn split_text(text: &str, font: &Font, max_w: u32) -> Vec<Line> {
    let mut lines = vec![Line::new()];

    let (space_w, line_h) = font
        .size()
        .call_into(|Dimensions { w, h }| (w as u32, h as u32));

    let delims = ['\n', '{'];
    let mut pos = 0;
    while let Some(mut idx) = text.find_from(delims, pos) {
        add_text(&mut lines, font, text, max_w, space_w, pos, idx);
        match text.chars().nth(idx) {
            Some('\n') => lines.push(Line::new()),
            Some('{') => {
                pos = idx + 1;
                idx = match text.find_from('}', pos) {
                    Some(idx) => idx,
                    None => panic!("split_text(): Unterminated '{{'"),
                };
                match &text[pos..idx] {
                    "b" => (),
                    "i" => {
                        match lines.last() {
                            Some(l) => {
                                if line_h > l.space(max_w) {
                                    lines.push(Line::new())
                                }
                            }
                            None => lines.push(Line::new()),
                        }
                        lines
                            .last_mut()
                            .expect("Failed to create new line")
                            .add_image(line_h);
                    }
                    _ => panic!("Unrecognized text wrap option: {}", &text[pos..idx]),
                }
            }
            _ => (),
        }
        pos = idx + 1;
    }
    add_text(&mut lines, font, text, max_w, space_w, pos, text.len());

    lines
}

// TODO: crashes if any text is empty
// TODO: doesn't work with spaces
pub fn render_text(
    r: &Renderer,
    am: &mut AssetManager,
    text: &str,
    font_data: &FontData,
    bounds: Rect,
    color: sdl2::SDL_Color,
    bkgrnd: sdl2::SDL_Color,
    ax: Align,
    ay: Align,
) -> (Texture, Vec<Rect>) {
    let font = am.get_font(font_data);
    let line_h = font.size().h as f32;
    let lines = split_text(text, font, bounds.w_i32() as u32);
    // println!("{lines:#?}");
    let text_r = Rect::new()
        .with_dim(
            lines.iter().max_by_key(|l| l.w).expect("No lines").w as f32,
            line_h * lines.len() as f32,
            Align::TopLeft,
            Align::TopLeft,
        )
        .with_rect_pos(bounds, ax, ay);
    let tex = Texture::new(r, text_r.w_i32() as u32, text_r.h_i32() as u32, bkgrnd);

    let mut imgs = Vec::new();
    let mut line_r = Rect {
        x: 0.0,
        y: 0.0,
        w: text_r.w,
        h: 0.0,
    };
    let mut y = 0.0;
    for line in lines.iter() {
        line_r.set_w(line.w as f32, ax);
        let mut x = line_r.x;
        for item in line.items.iter() {
            match item {
                LineItem::Text(t) => {
                    let rect = Rect {
                        x,
                        y,
                        w: t.w as f32,
                        h: line_h,
                    };
                    t.draw(r, &tex, rect, font, text, color);
                    x += t.w as f32;
                }
                LineItem::Image => {
                    imgs.push(Rect {
                        x,
                        y,
                        w: line_h,
                        h: line_h,
                    });
                    x += line_h;
                }
            }
        }
        y += line_h;
    }

    (tex, imgs)
}
