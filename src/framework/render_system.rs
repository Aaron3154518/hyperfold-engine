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
    mut comps: Vec<(&mut Elevation, &physics::Position, &Image)>,
    rs: &RenderSystem,
) {
    comps.sort_by(|(e1, ..), (e2, ..)| e1.cmp(&e2));
    for (_, pos, img) in comps {
        if let Some(tex) = img {
            rs.draw(&tex, std::ptr::null(), &pos.to_sdl_rect())
        }
    }
}
