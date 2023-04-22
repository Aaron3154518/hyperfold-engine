use ecs_lib::event;

#[event]
#[derive(Eq)]
enum CoreEvent {
    Update,
    Events,
    Render,
}

#[event]
#[derive(PartialEq, Eq)]
enum MyEvent {
    E1,
    E2(i32, i32),
    E3(String),
}

#[event]
pub enum OtherEvent {
    O1,
    O2(String, i32),
    O3,
}
