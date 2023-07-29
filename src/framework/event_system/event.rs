use crate::ecs::events::core;
use crate::utils::event;

#[macros::event]
struct Mouse(pub event::MouseButton);
#[macros::event]
struct Key(pub event::KeyButton);

#[macros::system]
fn on_event(_ev: &core::Events, e: &event::Event, events: &mut dyn crate::_engine::Events) {
    for m in [
        event::Mouse::Left,
        event::Mouse::Right,
        event::Mouse::Middle,
    ] {
        let mb = e.get_mouse(m);
        if !mb.no_action() {
            events.new_event(Mouse(mb.clone()));
        }
    }

    for (_, kb) in e.key_buttons.iter() {
        if !kb.no_action() {
            events.new_event(Key(kb.clone()));
        }
    }
}
