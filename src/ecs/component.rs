use ecs_lib::component;

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
    pub cnt: i32
}
