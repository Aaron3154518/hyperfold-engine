use ecs_lib::system;

use super::component::Component as Comp2;

#[system]
pub fn greet(
    comp: &mut super::super::MainComponent,
    comp2: &Comp2,
    comp4: &mut crate::ecs::component::MyComponent,
    comp3: &mut super::test::tmp::Component,
    res: &mut super::component::Resource,
) {
    println!(
        "#{}: Hi {} from {}. I have a message: The count is {} and \"{}\"",
        comp3.i, comp2.name, comp2.loc, res.cnt, comp4.msg
    );
    res.cnt += 1;
}

#[system]
pub fn super_mut(comp: &mut super::super::MainComponent, res: &mut super::component::Resource) {
    println!("Super Duper Mutable and the count is {}", res.cnt);
    res.cnt += 1;
}

#[system]
pub fn super_immut(comp: &super::super::MainComponent) {
    println!("Super Duper Immutable")
}

#[system]
pub fn super_duped(comp: &mut super::super::MainComponent) {
    println!("Super Duped")
}
