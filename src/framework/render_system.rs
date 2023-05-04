use std::cmp::Ordering;

use super::physics;
use crate::asset_manager::RenderSystem;
use crate::ecs::entity::Entity;
use crate::ecs::event;
use crate::utils::pointers;
use ecs_lib;

#[ecs_lib::component]
type Elevation = u8;

#[ecs_lib::component]
type Image = Option<pointers::TextureAccess>;

#[ecs_lib::system]
fn render(
    _e: &event::CoreEvent::Render,
    mut comps: Vec<(&mut Elevation, &Entity, &physics::Position, &Image)>,
    rs: &RenderSystem,
) {
    comps.sort_by(|(e1, id1, ..), (e2, id2, ..)| {
        let cmp = e1.cmp(&e2);
        if cmp == Ordering::Equal {
            id1.cmp(&id2)
        } else {
            cmp
        }
    });
    for (_, _, pos, img) in comps {
        if let Some(tex) = img {
            rs.draw(&tex, std::ptr::null(), &pos.to_sdl_rect())
        }
    }
}
