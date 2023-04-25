use ecs_lib;
use ecs_macros::Mut;

use crate::ecs::event::CoreEvent;
use crate::utils::event;

#[ecs_lib::event]
enum MouseEvent {
    Clicked(pub event::MouseButton),
    Event(pub event::MouseButton),
}

#[ecs_lib::system]
pub fn on_event(_ev: &CoreEvent::Events, e: &mut event::Event, events: &mut crate::EFoo) {
    for m in [
        event::Mouse::Left,
        event::Mouse::Right,
        event::Mouse::Middle,
    ] {
        let mb = e.get_mouse(m);
        if mb.clicked() {
            events.new_event(MouseEvent::Clicked(mb.clone()));
        }
        if !mb.no_action() {
            events.new_event(MouseEvent::Event(mb.clone()));
        }
    }
}

// TODO: Include own use path
#[ecs_lib::system]
pub fn on_left_click(ev: &super::event_system::MouseEvent::Clicked) {
    if ev.0.mouse == event::Mouse::Left {
        println!("{:#?}", ev.0);
    }
}
