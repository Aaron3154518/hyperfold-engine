use ecs_lib::event;

#[event]
enum CoreEvent {
    Update,
    Events,
    Render,
}

#[event]
enum MyEvent {
    E1,
    E2(pub i32, pub i32),
    E3 { pub name: String },
}

#[event]
enum OtherEvent {
    O1,
    O2(String, i32),
    O3,
}
