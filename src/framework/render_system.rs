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
    mut comps: Vec<(&Entity, &mut Elevation, &Entity, &physics::Position, &Image)>,
    rs: &RenderSystem,
) {
    comps.sort_by(|(id1, e1, ..), (id2, e2, ..)| {
        let cmp = e1.cmp(&e2);
        if cmp == Ordering::Equal {
            id1.cmp(&id2)
        } else {
            cmp
        }
    });
    for (_, _, _, pos, img) in comps {
        if let Some(tex) = img {
            rs.draw(&tex, std::ptr::null(), &pos.to_sdl_rect())
        }
    }
}

#[ecs_lib::system]
fn test_empty(_e: &event::CoreEvent::Render, mut v: Vec<(&Entity,)>) {
    println!("Hi: {}", v.len());
}
