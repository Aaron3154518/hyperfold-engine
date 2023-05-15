use crate::ecs::events::core;
use crate::utils::event;

pub mod inputs {
    use super::event;

    #[macros::event]
    struct Mouse(pub event::MouseButton);
    #[macros::event]
    struct Key(pub event::KeyButton);
}

#[macros::system]
pub fn on_event(_ev: &core::Events, e: &event::Event, events: &mut dyn crate::_engine::AddEvent) {
    for m in [
        event::Mouse::Left,
        event::Mouse::Right,
        event::Mouse::Middle,
    ] {
        let mb = e.get_mouse(m);
        if !mb.no_action() {
            events.new_event(inputs::Mouse(mb.clone()));
        }
    }

    for (key, kb) in e.key_buttons.iter() {
        if !kb.no_action() {
            events.new_event(inputs::Key(kb.clone()));
        }
    }
}
