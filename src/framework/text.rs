use shared::util::Call;

use crate::utils::{
    colors::{BLACK, GRAY},
    rect::{Align, Dimensions, Rect},
    util::{FindFrom, FloatMath},
};

use super::{
    font::FontTrait,
    render_data::RenderData,
    renderer::RendererTrait,
    texture::{Texture, TextureTrait},
};

// Text
pub struct Text {
    start: usize,
    end: usize,
    w: u32,
}

impl Text {
    pub fn draw(
        &self,
        r: &impl RendererTrait,
        tex: &Texture,
        rect: Rect,
        font: &impl FontTrait,
        text: &str,
    ) {
        let text_tex = Texture::from_surface(r, font.render(&text[self.start..self.end], BLACK));
        let text_rect = text_tex.min_rect_align(rect, Align::Center, Align::Center);
        tex.draw(r, RenderData::from(&text_tex).set_pos(text_rect));
    }
}

// Line
enum LineItem {
    Text(Text),
    Image,
}

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
    font: &impl FontTrait,
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

pub fn split_text(text: &str, font: &impl FontTrait, max_w: u32) -> Vec<Line> {
    let mut lines = vec![Line::new()];

    let (space_w, line_h) = font
        .size()
        .call_into(|Dimensions { w, h }| (w as u32, h as u32));

    let delims = ['\n', '{'];
    let mut pos = 0;
    while let Some(idx) = text.find_from(delims, pos) {
        add_text(&mut lines, font, text, max_w, space_w, pos, idx);
        match text.chars().nth(idx) {
            Some('\n') => lines.push(Line::new()),
            Some('{') => {
                pos = idx + 1;
                let idx = match text.find_from('}', pos) {
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

pub fn render_text(
    r: &impl RendererTrait,
    text: &str,
    font: &impl FontTrait,
    rect: Rect,
    ax: Align,
    ay: Align,
) -> Texture {
    let Dimensions { h: line_h, .. } = font.size();
    let lines = split_text(text, font, rect.w_i32() as u32);
    let mut text_r = Rect {
        x: 0.0,
        y: 0.0,
        w: rect.w,
        h: line_h as f32 * lines.len() as f32,
    };
    text_r.copy_pos(rect, ax, ay);
    let tex = Texture::new(r, text_r.w_i32(), text_r.h_i32(), GRAY);

    let num_imgs = lines.iter().fold(0, |s, l| s + l.img_cnt);

    // TODO: get images

    // Set to 1 for now
    let scale = Dimensions {
        w: text_r.w / rect.w,
        h: text_r.h / line_h as f32 / lines.len() as f32,
    };
    let mut line_r = Rect {
        x: 0.0,
        y: 0.0,
        w: rect.w,
        h: line_h as f32,
    };
    let img_w = (line_r.h * scale.h).round_i32() as u32;
    for line in lines.iter() {
        line_r.set_w(line.w as f32, ax);
        let mut x = line_r.x_i32() as u32;
        for item in line.items.iter() {
            match item {
                LineItem::Text(t) => {
                    let rect = Rect {
                        x: x as f32,
                        y: line_r.y,
                        w: t.w as f32,
                        h: line_r.h,
                    };
                    t.draw(r, &tex, rect, font, text);
                    x += t.w;
                }
                LineItem::Image => {
                    // TODO: Update image width
                    x += img_w;
                }
            }
        }
        line_r.move_by(0.0, line_r.h);
    }

    // TODO: update images

    tex
}

// void TextData::draw() {
//     if (updateText) {
//         int lineH = TTF_FontHeight(mFont.get());
//         mLines = splitText(mText, mFont, mRect.W());
//         TextureBuilder tex(mRect.W(), lineH * mLines.size());

//         Rect r = Rect(0, 0, mRect.W(), lineH * mLines.size());
//         DimensionsF scale{r.w() / mRect.w(), r.h() / lineH / mLines.size()};
//         r.setPos(mRect, mAlignX, mAlignY);

//         size_t numImgs = 0;
//         for (auto& line : mLines) {
//             numImgs += line.numImgs();
//         }
// TODO:
//         size_t currSize = mImgEntities.size();
//         mImgEntities.resize(numImgs);
//         for (size_t i = currSize; i < mImgEntities.size(); i++) {
//             mImgEntities.at(i) = GameObjects::New<ImageEntity>();
//             mImgEntities.at(i)->setElevation(
//                 getComponent<ElevationComponent>().get());
//         }
//         auto imgEntIt = mImgEntities.begin();

//         Rect lineR(0, 0, mRect.W(), lineH);
//         float imgW = lineR.h() * scale.h;
//         for (auto& line : mLines) {
//             lineR.setWidth(line.w(), mAlignX);
//             float x = lineR.x();
//             auto textIt = line.mText.begin();
//             for (auto type : line.mTypes) {
//                 switch (type) {
//                     case Line::Type::TEXT: {
//                         Text& t = *(textIt++);
//                         t.draw(tex, Rect(x, lineR.y(), t.w, lineR.h()), mFont,
//                                mText);
//                         x += t.w;
//                     } break;
//                     case Line::Type::IMAGE: {
// TODO:
//                         (*(imgEntIt++))
//                             ->setRect(Rect(x * scale.w + r.x(),
//                                            lineR.y() * scale.h + r.y(), imgW,
//                                            imgW));
//                         x += imgW;
//                     } break;
//                 };
//             }
//             lineR.move(0, lineR.h());
//         }

//         getComponent<PositionComponent>().set(r);
//         getComponent<SpriteComponent>().setTexture(tex.getTexture());

//         updateText = false;
//     }
//     if (updateImgs) {
// TODO:
//         auto imgIt = mImgs.begin();
//         for (auto it = mImgEntities.begin(); it != mImgEntities.end(); ++it) {
//             if (imgIt != mImgs.end()) {
//                 (*it)->setImg(*imgIt);
//             } else {
//                 (*it)->setImg(SpriteData());
//             }
//         }

//         updateImgs = false;
//     }
// }
