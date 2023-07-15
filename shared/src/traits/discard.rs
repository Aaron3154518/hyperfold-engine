// Discard a value
pub trait Discard {
    fn discard(self) -> ();
}

impl<T> Discard for T {
    fn discard(self) -> () {}
}
