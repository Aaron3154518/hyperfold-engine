use ecs_lib::system;

use crate::ecs::event::CoreEvent;
use crate::utils::event::{Event, Mouse};

#[system]
pub fn on_event(ev: &CoreEvent::Events, e: &mut Event) {
    if e.get_mouse(Mouse::Left).clicked() {
        println!("Clicked")
    }
}
