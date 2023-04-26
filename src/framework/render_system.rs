use super::physics;
use crate::asset_manager::RenderSystem;
use crate::draw;
use crate::ecs::event;
use crate::utils::pointers;
use ecs_lib;

#[ecs_lib::component]
type Image = Option<pointers::TextureAccess>;

#[ecs_lib::system]
fn render(
    _e: &event::CoreEvent::Render,
    pos: &physics::Position,
    img: &super::render_system::Image,
    rs: &RenderSystem,
) {
    if let Some(tex) = img {
        rs.draw(&tex, std::ptr::null(), &pos.to_sdl_rect())
    }
}
