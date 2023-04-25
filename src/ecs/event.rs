use ecs_lib::event;

#[event]
enum CoreEvent {
    Update(u32),
    Events,
    Render,
}

#[event]
enum MyEvent {
    E1,
    E2(i32, i32),
    E3 { name: String },
}

#[event]
enum OtherEvent {
    O1,
    O2(String, i32),
    O3,
}
