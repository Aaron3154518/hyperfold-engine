use crate::_engine::{AddEvent, Entity};
use crate::components;
use crate::ecs::events::core;
use crate::utils::event::{self, Event, Mouse};

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

    #[macros::event]
    struct DragStart(pub Entity);
    #[macros::event]
    struct Drag {
        pub eid: Entity,
        pub mouse_x: i32,
        pub mouse_y: i32,
        pub mouse_dx: i32,
        pub mouse_dy: i32,
    }
    #[macros::event]
    struct DragEnd(pub Entity);
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

#[derive(Copy, Clone)]
pub enum DragTrigger {
    DelayMs(u32),
    OnMove,
}

#[macros::component]
struct Drag {
    pub trigger: DragTrigger,
    // dragging: bool,
}

impl Drag {
    pub fn new(trigger: DragTrigger) -> Self {
        Self {
            trigger,
            // dragging: false,
        }
    }
}

components!(
    DragArgs,
    e: &'a Elevation,
    pos: &'a Position,
    drag: &'a mut Drag
);

#[macros::global]
struct DragState {
    dragging: Option<Entity>,
    hold_target: Option<(Entity, DragTrigger, Mouse)>,
}

impl DragState {
    pub fn new() -> Self {
        Self {
            dragging: None,
            hold_target: None,
        }
    }
}

// TODO: lock mouse clicks
#[macros::system]
fn trigger_drag(
    m: &inputs::Mouse,
    entities: Vec<DragArgs>,
    events: &mut dyn AddEvent,
    drag_state: &mut DragState,
) {
    if m.0.up() {
        if let Some(eid) = drag_state.dragging {
            events.new_event(inputs::DragEnd(eid));
            drag_state.dragging = None;
            drag_state.hold_target = None;
        }
    } else if m.0.down() {
        for DragArgs { eid, e, pos, drag } in
            sort_elevation(entities, |t| t.e, |t| t.eid, Order::Desc)
        {
            if pos.0.contains_point_i32(m.0.click_pos) {
                if let Some(eid) = drag_state.dragging {
                    events.new_event(inputs::DragEnd(eid));
                    drag_state.dragging = None;
                }
                drag_state.hold_target = Some((*eid, drag.trigger, m.0.mouse));
                return;
            }
        }
    }
}

#[macros::system]
fn drag(_: &core::Events, event: &Event, events: &mut dyn AddEvent, drag_state: &mut DragState) {
    if let Some(eid) = drag_state.dragging {
        if event.mouse_moved() {
            events.new_event(inputs::Drag {
                eid,
                mouse_x: event.mouse.x,
                mouse_y: event.mouse.y,
                mouse_dx: event.mouse_delta.x,
                mouse_dy: event.mouse_delta.y,
            });
        }
    } else if let Some((eid, trigger, mouse)) = drag_state.hold_target {
        if match trigger {
            DragTrigger::OnMove => event.mouse_moved(),
            DragTrigger::DelayMs(d) if event.get_mouse(mouse).duration >= d => true,
            DragTrigger::DelayMs(_) => {
                if event.mouse_moved() {
                    drag_state.hold_target = None;
                }
                false
            }
        } {
            events.new_event(inputs::DragStart(eid));
            drag_state.dragging = Some(eid);
            drag_state.hold_target = None;
        }
    }
}
