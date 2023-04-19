use ecs_lib::system;

use super::component::Component as Comp2;

#[system]
pub fn greet(
    comp: &mut super::super::MainComponent,
    comp2: &Comp2,
    comp4: &mut crate::ecs::component::MyComponent,
    comp3: &mut super::test::tmp::Component,
) {
    println!(
        "#{}: Hi {} from {}. I have a message: \"{}\"",
        comp3.i, comp2.name, comp2.loc, comp4.msg
    )
}

#[system]
pub fn super_mut(comp: &mut super::super::MainComponent) {
    println!("Super Duper Mutable")
}

#[system]
pub fn super_immut(comp: &super::super::MainComponent) {
    println!("Super Duper Immutable")
}

#[system]
pub fn super_duped(comp: &mut super::super::MainComponent) {
    println!("Super Duped")
}
