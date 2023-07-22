use crate::_engine::{AddEvent, Entity};
use crate::components;
use crate::ecs::events::core;
use crate::framework::{
    physics::Position,
    render_system::{sort_elevation, Elevation, Order},
};
use crate::utils::event::{self, Event, Mouse};

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

// Drag components/globals
#[macros::component]
#[derive(Copy, Clone)]
enum DragTrigger {
    DelayMs(u32),
    OnMove,
}

struct DragHoldTarget {
    eid: Entity,
    trigger: DragTrigger,
    mouse: Mouse,
}

#[macros::global]
struct DragState {
    dragging: Option<Entity>,
    hold_target: Option<DragHoldTarget>,
}

impl DragState {
    pub fn new() -> Self {
        Self {
            dragging: None,
            hold_target: None,
        }
    }
}

// Handle mouse events
components!(
    MouseArgs,
    e: &'a Elevation,
    pos: &'a Position,
    drag_trigger: Option<&'a DragTrigger>
);

#[macros::system]
fn mouse_event(
    m: &super::events::Mouse,
    entities: Vec<MouseArgs>,
    events: &mut dyn AddEvent,
    drag_state: &mut DragState,
) {
    // Finished dragging; no more mouse events
    if m.0.up() {
        if let Some(eid) = drag_state.dragging {
            events.new_event(DragEnd(eid));
            drag_state.dragging = None;
            drag_state.hold_target = None;
            return;
        }
    }

    // Get entity under mouse
    let target = sort_elevation(entities, |t| t.e, |t| t.eid, Order::Desc)
        .into_iter()
        .find(|MouseArgs { pos, .. }| pos.0.contains_point_i32(m.0.click_pos));

    // New drag target
    if m.0.down() {
        match target {
            Some(MouseArgs {
                eid,
                drag_trigger: Some(trigger),
                ..
            }) => {
                // End previous drag (shouldn't be one)
                if let Some(eid) = drag_state.dragging {
                    events.new_event(DragEnd(eid));
                    drag_state.dragging = None;
                }
                drag_state.hold_target = Some(DragHoldTarget {
                    eid: *eid,
                    trigger: *trigger,
                    mouse: m.0.mouse,
                });
            }
            _ => (),
        }
    // Mouse click
    } else if m.0.clicked() {
        events.new_event(Click {
            eid: target.map(|t| *t.eid),
            button: m.0,
        });
    }
}

#[macros::system]
fn drag(_: &core::Events, event: &Event, events: &mut dyn AddEvent, drag_state: &mut DragState) {
    if let Some(eid) = drag_state.dragging {
        if event.mouse_moved() {
            events.new_event(Drag {
                eid,
                mouse_x: event.mouse.x,
                mouse_y: event.mouse.y,
                mouse_dx: event.mouse_delta.x,
                mouse_dy: event.mouse_delta.y,
            });
        }
    } else if let Some(DragHoldTarget {
        eid,
        trigger,
        mouse,
    }) = drag_state.hold_target
    {
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
            events.new_event(DragStart(eid));
            events.new_event(Drag {
                eid,
                mouse_x: event.mouse.x,
                mouse_y: event.mouse.y,
                mouse_dx: 0,
                mouse_dy: 0,
            });
            drag_state.dragging = Some(eid);
            drag_state.hold_target = None;
        }
    }
}
