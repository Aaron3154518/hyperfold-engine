use crate::ecs;

#[ecs::event]
enum CoreEvent {
    Update(u32),
    Events,
    Render,
}
