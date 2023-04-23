use std::{any::TypeId, hash::Hash};

use ecs_lib::{component, event};

#[derive(PartialEq, Eq, Hash)]
pub struct TypeIdx {
    id: TypeId,
    vi: usize,
}

impl TypeIdx {
    pub const fn new<T: 'static>(i: usize) -> Self {
        Self {
            id: TypeId::of::<T>(),
            vi: i,
        }
    }
}

#[event]
enum CoreEvent {
    Update,
    Events,
    Render,
}

#[event]
enum MyEvent {
    E1,
    E2(i32, i32),
    E3(String),
}

#[event]
enum OtherEvent {
    O1,
    O2(String, i32),
    O3,
}

#[component(Global)]
pub struct CurrEvent {
    event: crate::EFoo,
}

impl CurrEvent {
    pub fn new() -> Self {
        Self {
            event: crate::EFoo::from(CoreEvent::Update),
        }
    }

    pub fn from(e: crate::EFoo) -> Self {
        Self { event: e }
    }

    pub fn get(&self) -> &crate::EFoo {
        &self.event
    }

    pub fn get_mut(&mut self) -> &mut crate::EFoo {
        &mut self.event
    }
}

#[component(Global)]
pub struct EventBus {
    bus: Vec<crate::EFoo>,
}

// TODO: vector of stacks (process in order, children then parent)
impl EventBus {
    pub fn new() -> Self {
        Self { bus: Vec::new() }
    }

    pub fn push<T>(&mut self, t: T)
    where
        crate::EFoo: From<T>,
    {
        self.bus.push(crate::EFoo::from(t))
    }

    pub fn reset(&mut self) {
        self.bus = Vec::new();
        for e in [CoreEvent::Render, CoreEvent::Events, CoreEvent::Update] {
            self.push(e);
        }
    }

    pub fn pop(&mut self) -> Option<crate::EFoo> {
        self.bus.pop()
    }
}
