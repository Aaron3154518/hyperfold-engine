pub mod components;
pub mod entities;
pub mod events;
pub mod systems;

pub trait ManagerTrait {
    fn new() -> Self;

    fn run(&mut self);
}
