pub trait Mut<T> {
    fn new_event(&mut self, t: T);

    fn get_event<'a>(&'a self) -> Option<&'a T>;
}

pub trait ComponentManager<E, T> {
    fn add_component(&mut self, e: E, t: T);
}
