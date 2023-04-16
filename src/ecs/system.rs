use ecs_lib::system;

use super::component::Component;

#[system]
pub fn greet(comp: &Component, comp2: &&&super::component::Component) {
    println!("Hi {} from {}", comp.name, comp.loc)
}
