pub type Event<'a, T> = &'a T;

pub trait AddEvent<T> {
    fn new_event(&mut self, t: T);

    fn get_event<'a>(&'a self) -> Option<&'a T>;
}

pub mod core {
    #[macros::event]
    pub struct Update(pub u32);
    #[macros::event]
    pub struct Events;
    #[macros::event]
    pub struct PreRender;
    #[macros::event]
    pub struct Render;
}
