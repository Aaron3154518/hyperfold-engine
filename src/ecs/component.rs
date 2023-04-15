use ecs_lib::component;

#[component]
pub struct Component {
    pub name: &'static str,
    pub loc: &'static str,
}
