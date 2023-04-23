use ecs_lib::system;

use super::component::Component as Comp2;
use super::event::{CoreEvent, MyEvent};

#[system(MyEvent::E2)]
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

#[system(CoreEvent::Update)]
pub fn super_mut(
    comp: &mut super::super::MainComponent,
    res: &mut super::component::Resource,
    em: &mut super::event::EventBus,
) {
    println!("Super Duper Mutable and the count is {}", res.cnt);
    res.cnt += 1;
    em.push(MyEvent::E2(1, 1));
}

#[system(CoreEvent::Render)]
pub fn super_immut(comp: &super::super::MainComponent) {
    println!("Super Duper Immutable on Render")
}

#[system]
pub fn super_duped(comp: &super::super::MainComponent) {
    println!("Super Duped")
}
