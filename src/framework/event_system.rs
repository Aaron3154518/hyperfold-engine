use crate::_engine::AddEvent;
use crate::components;
use crate::ecs::events::core;
use crate::utils::event;

use super::physics::Position;
use super::render_system::{sort_elevation, Elevation, Order};

pub mod inputs {
    use crate::_engine::Entity;

    use super::event;

    #[macros::event]
    struct Mouse(pub event::MouseButton);
    #[macros::event]
    struct Key(pub event::KeyButton);

    #[macros::event]
    struct Click {
        pub eid: Option<Entity>,
        pub button: event::MouseButton,
    }

    impl Click {
        pub fn is_me(&self, eid: &Entity) -> bool {
            self.eid.is_some_and(|id| &id == eid)
        }
    }
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

    for (_, kb) in e.key_buttons.iter() {
        if !kb.no_action() {
            events.new_event(inputs::Key(kb.clone()));
        }
    }
}

components!(OnClickArgs, e: &'a Elevation, pos: &'a Position);

#[macros::system]
fn on_click(m: &inputs::Mouse, entities: Vec<OnClickArgs>, events: &mut dyn AddEvent) {
    if !m.0.clicked() {
        return;
    }
    for OnClickArgs { eid, pos, .. } in sort_elevation(entities, |t| t.e, |t| t.eid, Order::Desc) {
        if pos.0.contains_point_i32(m.0.click_pos) {
            events.new_event(inputs::Click {
                eid: Some(*eid),
                button: m.0,
            });
            return;
        }
    }
    events.new_event(inputs::Click {
        eid: None,
        button: m.0,
    });
}
