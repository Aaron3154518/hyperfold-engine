use crate::ecs;
use crate::includes::*;

use crate::ecs::events::CoreEvent;
use crate::utils::event;

#[ecs::event]
enum Events {
    Mouse(event::MouseButton),
    Key(event::KeyButton),
}

#[ecs::system]
pub fn on_event(_ev: &CoreEvent::Events, e: &event::Event, events: &mut dyn crate::EFooT) {
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

    for (key, kb) in e.key_buttons.iter() {
        if !kb.no_action() {
            events.new_event(Events::Key(kb.clone()));
        }
    }
}
