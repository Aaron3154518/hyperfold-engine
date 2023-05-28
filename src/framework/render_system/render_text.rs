use uuid::Uuid;

use crate::ecs::entities::Entity;

use super::{font::FontData, RenderComponent};

pub enum TextImage {
    Image(RenderComponent),
    Reference(Entity),
}

pub struct RenderText {
    font_data: FontData,
    tex_id: Uuid,
    imgs: Vec<TextImage>,
}
