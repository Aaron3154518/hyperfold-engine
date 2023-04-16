use ecs_lib::system;

use super::component::Component;
use super::test::tmp::Component as Comp2;

#[system]
pub fn greet(
    comp: &crate::MainComponent,
    comp1: &&&super::component::Component,
    comp2: Component,
    comp3: &super::test::tmp::Component,
    comp4: Comp2,
    comp5: crate::ecs::component::MyComponent,
) {
    println!("Hi {} from {}", comp1.name, comp1.loc)
}
