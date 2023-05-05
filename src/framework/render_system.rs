use std::cmp::Ordering;

use super::physics;
use crate::asset_manager::RenderSystem;
use crate::ecs::component::Components;
use crate::ecs::entity::Entity;
use crate::ecs::event;
use crate::utils::pointers;
use ecs_lib;

#[ecs_lib::component]
struct Elevation(pub u8);

#[ecs_lib::component]
struct Image(pub Option<pointers::TextureAccess>);

#[ecs_lib::system]
fn render(
    _e: &event::CoreEvent::Render,
    mut comps: Components<(&Entity, &mut Elevation, &Entity, &physics::Position, &Image)>,
    rs: &RenderSystem,
) {
    comps.sort_by(|(id1, e1, ..), (id2, e2, ..)| {
        let cmp = e1.0.cmp(&e2.0);
        if cmp == Ordering::Equal {
            id1.cmp(&id2)
        } else {
            cmp
        }
    });
    for (_, _, _, pos, img) in comps {
        if let Image(Some(tex)) = img {
            rs.draw(&tex, std::ptr::null(), &pos.0.to_sdl_rect())
        }
    }
}
