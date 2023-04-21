use ecs_lib::event;
use ecs_macros::events;

#[event]
pub enum MyEvent {
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

events!(
    EFoo,
    MyEvent0(crate::ecs::event::MyEvent),
    OtherEvent0(crate::ecs::event::OtherEvent),
    OtherEvent1(crate::ecs::component::OtherEvent)
);
