use super::physics;
use crate::asset_manager::RenderSystem;
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
    mut comps: Vec<(&Elevation, &physics::Position, &Image)>,
    // pos: &physics::Position,
    // img: &super::render_system::Image,
    rs: &RenderSystem,
) {
    comps.sort_by_key(|(e, ..)| **e);
    for (_, pos, img) in comps {
        if let Some(tex) = img {
            rs.draw(&tex, std::ptr::null(), &pos.to_sdl_rect())
        }
    }
}
