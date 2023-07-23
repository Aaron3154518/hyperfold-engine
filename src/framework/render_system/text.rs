use itertools::{Itertools, PeekingNext};
use shared::traits::{Call, CollectVecInto, FindFrom};

use crate::{
    sdl2,
    utils::rect::{Align, Dimensions, Rect},
};

use super::{
    drawable::Canvas,
    font::{Font, FontData},
    render_data::{Fit, RectMode, RenderDataBuilderTrait, RenderTexture},
    AssetManager, Renderer, Texture,
};

// Text
#[derive(Debug)]
pub struct Text {
    tok_idx: usize,
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
        tokens: &Vec<TextToken>,
        color: sdl2::SDL_Color,
    ) {
        match tokens.get(self.tok_idx) {
            Some(TextToken::Text(text)) => {
                let text_tex =
                    r.create_texture_from_surface(font.render(&text[self.start..self.end], color));
                tex.draw(
                    r,
                    &mut RenderTexture::new(Some(text_tex)).with_dest(
                        rect,
                        RectMode::Absolute,
                        Fit::fit_dest(),
                        Align::Center,
                        Align::Center,
                    ),
                )
            }
            _ => panic!("Invalid text token index"),
        }
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
    line: &mut Line,
    tok_idx: usize,
    text: &str,
    font: &Font,
    max_w: u32,
    space_w: u32,
    pos1: usize,
    pos2: usize,
) {
    if pos1 >= pos2 {
        return;
    }

    let segment = &text[pos1..pos2];
    let (width, count) = font
        .measure(segment, line.space(max_w))
        .call_into(|(w, c)| (w as u32, c as usize));

    if count == pos2 - pos1 {
        // Fit entire text onto line
        line.add_text(Text {
            tok_idx,
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
                line.add_text(Text {
                    tok_idx,
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
                    line.add_text(Text {
                        tok_idx,
                        start: pos1,
                        end: pos1 + count,
                        w: width,
                    });
                    pos1 + count + 1
                }
            }
        };
        lines.push(std::mem::replace(line, Line::new()));
        add_text(
            lines, line, tok_idx, text, font, max_w, space_w, new_start, pos2,
        );
    }
}

pub fn split_text(tokens: &Vec<TextToken>, font: &Font, max_w: Option<u32>) -> Vec<Line> {
    let (space_w, line_h) = font
        .size()
        .call_into(|Dimensions { w, h }| (w as u32, h as u32));

    let mut lines = Vec::new();
    let mut line = Line::new();
    match max_w {
        Some(max_w) => {
            for (tok_idx, token) in tokens.into_iter().enumerate() {
                match token {
                    TextToken::Text(text) => add_text(
                        &mut lines,
                        &mut line,
                        tok_idx,
                        text,
                        font,
                        max_w,
                        space_w,
                        0,
                        text.len(),
                    ),
                    TextToken::Image => {
                        if line_h > line.space(max_w) {
                            lines.push(std::mem::replace(&mut line, Line::new()))
                        }
                        line.add_image(line_h);
                    }
                    TextToken::NewLine => lines.push(std::mem::replace(&mut line, Line::new())),
                }
            }
        }
        None => {
            for (tok_idx, token) in tokens.into_iter().enumerate() {
                match token {
                    TextToken::Text(text) => line.add_text(Text {
                        tok_idx,
                        start: 0,
                        end: text.len(),
                        w: font.size_text(text).w.max(0) as u32,
                    }),
                    TextToken::Image => line.add_image(line_h),
                    TextToken::NewLine => lines.push(std::mem::replace(&mut line, Line::new())),
                }
            }
        }
    };

    if lines.is_empty() || line.w > 0 {
        lines.push(line);
    }

    lines
}

#[derive(Debug)]
pub enum TextToken {
    Text(String),
    Image,
    NewLine,
}

pub fn parse_text(text: &str) -> Vec<TextToken> {
    let mut tokens = Vec::new();
    for (i, text) in text.split("\n").enumerate() {
        if i > 0 {
            tokens.push(TextToken::NewLine);
        }

        let mut string = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        let mut idx = 0;
        let mut last_sep = None;
        while let Some((i, ch)) = chars.find_position(|ch| ch == &'[' || ch == &']') {
            string.push_str(&text[idx..idx + i]);
            idx += i + 1;
            match chars.peek() {
                // Escaped ch
                Some(ch2) if ch2 == &ch => {
                    chars.next();
                    string.push(ch);
                    idx += 1;
                }
                _ => {
                    // Check control sequence
                    if (ch == '[' && last_sep.unwrap_or(']') != ']')
                        || (ch == ']' && last_sep != Some('['))
                    {
                        panic!("Unexpected: '{ch}'");
                    }
                    if ch == ']' {
                        match string.as_str() {
                            "i" => tokens.push(TextToken::Image),
                            "b" => (),
                            str => panic!("Unexpected command sequence: [{str}]"),
                        }
                    } else {
                        tokens.push(TextToken::Text(string));
                    }
                    last_sep = Some(ch);
                    string = String::with_capacity(text.len() - idx);
                }
            }
        }
        // Make sure '[' is terminated
        if last_sep == Some('[') {
            panic!("Unterminated '['");
        }
        // Handle leftover string
        string.push_str(&text[idx..]);
        if !string.is_empty() {
            tokens.push(TextToken::Text(string));
        }
    }

    tokens
}

// TODO: crashes if any text is empty
// TODO: doesn't work with spaces
pub fn render_text(
    r: &Renderer,
    am: &mut AssetManager,
    tokens: &Vec<TextToken>,
    font_data: &FontData,
    max_w: Option<u32>,
    bounds: Rect,
    color: sdl2::SDL_Color,
    bkgrnd: sdl2::SDL_Color,
    ax: Align,
    ay: Align,
) -> (Texture, Vec<Rect>) {
    let font = am.get_font(font_data);
    let line_h = font.size().h as f32;
    let lines = split_text(tokens, font, max_w);
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
                    t.draw(r, &tex, rect, font, tokens, color);
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
