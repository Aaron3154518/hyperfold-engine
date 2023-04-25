use ecs_lib;
use ecs_macros::Mut;

use crate::ecs::event::CoreEvent;
use crate::utils::event;

#[ecs_lib::event]
enum Events {
    Mouse(pub event::MouseButton),
}

#[ecs_lib::system]
pub fn on_event(_ev: &CoreEvent::Events, e: &mut event::Event, events: &mut crate::EFoo) {
    for m in [
        event::Mouse::Left,
        event::Mouse::Right,
        event::Mouse::Middle,
    ] {
        let mb = e.get_mouse(m);
        if !mb.no_action() {
            events.new_event(Events::Mouse(mb.clone()));
        }
    }
}
