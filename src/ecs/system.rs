use ecs_lib::system;
use ecs_macros::Mut;

use super::component::Component as Comp2;
use super::event::{CoreEvent, MyEvent};

#[system]
pub fn empty(ev: &CoreEvent::Events, res: &super::component::Resource) {
    println!("I am empty! {}", res.cnt);
}

#[system]
pub fn greet(
    ev2: &MyEvent::E2,
    _comp: &mut super::super::MainComponent,
    comp2: &Comp2,
    comp4: &mut crate::ecs::component::MyComponent,
    comp3: &mut super::test::tmp::Component,
    res: &mut super::component::Resource,
) {
    res.cnt += ev2.0 * ev2.1;
    println!(
        "#{}: Hi {} from {}. I have a message: The count is {} and \"{}\"",
        comp3.i, comp2.name, comp2.loc, res.cnt, comp4.msg
    );
}

#[system]
pub fn super_mut(
    ev: &CoreEvent::Update,
    _comp: &mut super::super::MainComponent,
    res: &mut super::component::Resource,
    em: &mut crate::EFoo,
) {
    res.cnt += 1;
    println!("Super Duper Mutable and the count is {}", res.cnt);
    em.new_event(MyEvent::E2(2, 34));
}

#[system]
pub fn super_immut(ev: &CoreEvent::Render, _comp: &super::super::MainComponent) {
    println!("Super Duper Immutable on Render")
}

#[system]
pub fn super_duped(_comp: &super::super::MainComponent) {
    println!("Super Duped")
}
