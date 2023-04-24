use std::collections::VecDeque;

use ecs_lib::{component, event, system_manager};

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

pub trait PushEvent<T: 'static> {
    fn push(&mut self, t: T);

    fn add_system(&mut self, f: Box<dyn Fn(&mut crate::Foo, &T)>);
}

pub trait PopRun {
    fn is_empty(&self) -> bool;

    fn pop_run(&mut self, cm: &mut crate::Foo) -> bool;
}

pub struct RunEvent<'a, T> {
    e: T,
    funcs: Vec<&'a Box<dyn Fn(&mut crate::Foo, &T)>>,
}

impl<'a, T> PopRun for RunEvent<'a, T> {
    fn is_empty(&self) -> bool {
        self.funcs.is_empty()
    }

    fn pop_run(&mut self, cm: &mut crate::Foo) -> bool {
        self.funcs.pop().map_or(false, |f| {
            (f)(cm, &self.e);
            true
        })
    }
}

system_manager!(SystemManager);

// TODO fix encapsulation
// TODO don't have to import PushEvent
#[component(Global)]
pub struct EventBus {
    pub manager: SystemManager,
    pub stack: Vec<VecDeque<Box<dyn PopRun>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            manager: SystemManager::new(),
            stack: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.stack = vec![VecDeque::new()];
        self.push(CoreEvent::Update);
        self.push(CoreEvent::Events);
        self.push(CoreEvent::Render);
    }
}

impl PopRun for EventBus {
    fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    fn pop_run(&mut self, cm: &mut crate::Foo) -> bool {
        // Remove empty queues and PopRuns
        while self.stack.last_mut().is_some_and(|queue| {
            while queue.front_mut().is_some_and(|poprun| poprun.is_empty()) {
                queue.pop_front();
            }
            queue.is_empty()
        }) {
            self.stack.pop();
        }
        // Add a new queue
        self.stack.push(VecDeque::new());
        // Run top function
        let i = self.stack.len() - 2;
        self.stack
            .get_mut(i)
            .and_then(|queue| queue.front_mut())
            .map_or(false, |e| e.pop_run(cm))
    }
}
