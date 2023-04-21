use ecs_lib::{component, event};

#[component]
pub struct Component {
    pub name: &'static str,
    pub loc: &'static str,
}

#[component]
pub struct MyComponent {
    pub msg: String,
}

#[component(Global)]
pub struct Resource {
    pub cnt: i32,
}

impl Resource {
    pub fn new() -> Self {
        Self { cnt: 0 }
    }
}

#[event]
pub enum OtherEvent {
    O1,
    O2(String, i32),
    O3,
}
