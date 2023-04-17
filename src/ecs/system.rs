use ecs_lib::system;

use super::component::Component as Comp2;

#[system]
pub fn greet(
    comp: &crate::MainComponent,
    comp2: &Comp2,
    comp3: &mut super::test::tmp::Component,
    comp4: crate::ecs::component::MyComponent,
) {
    println!(
        "#{}: Hi {} from {}. I have a message: \"{}\"",
        comp3.i, comp2.name, comp2.loc, comp4.msg
    )
}
