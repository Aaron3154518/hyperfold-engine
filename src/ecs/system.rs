use ecs_lib::system;
use ecs_macros::{match_event, To};

use super::component::Component as Comp2;
use super::event::{CoreEvent, MyEvent};

#[system(CoreEvent::Events)]
pub fn empty(res: &super::component::Resource) {
    println!("I am empty! {}", res.cnt);
}

#[system(MyEvent::E1, MyEvent::E2)]
pub fn greet(
    _comp: &mut super::super::MainComponent,
    comp2: &Comp2,
    comp4: &mut crate::ecs::component::MyComponent,
    comp3: &mut super::test::tmp::Component,
    res: &mut super::component::Resource,
    ev: &super::event::CurrEvent,
) {
    match_event!(ev, MyEvent::E1, {
        res.cnt += 10;
    });
    match_event!(ev, MyEvent::E2(i1, i2), { res.cnt += i1 * i2 });
    println!(
        "#{}: Hi {} from {}. I have a message: The count is {} and \"{}\"",
        comp3.i, comp2.name, comp2.loc, res.cnt, comp4.msg
    );
}

#[system(CoreEvent::Update)]
pub fn super_mut(
    _comp: &mut super::super::MainComponent,
    res: &mut super::component::Resource,
    em: &mut super::event::EventBus,
) {
    res.cnt += 1;
    println!("Super Duper Mutable and the count is {}", res.cnt);
    em.push(MyEvent::E2(2, 29));
    em.push(MyEvent::E1);
}

#[system(CoreEvent::Render)]
pub fn super_immut(_comp: &super::super::MainComponent) {
    println!("Super Duper Immutable on Render")
}

#[system]
pub fn super_duped(_comp: &super::super::MainComponent) {
    println!("Super Duped")
}
