pub trait AddEvent<T> {
    fn new_event(&mut self, t: T);

    fn get_event<'a>(&'a self) -> Option<&'a T>;
}

pub trait SetState<T> {
    fn set_state(&mut self, t: T);
}

pub mod core {
    #[macros::event]
    struct Update(pub u32);
    #[macros::event]
    struct Events;
    #[macros::event]
    struct PreRender;
    #[macros::event]
    struct Render;
}
