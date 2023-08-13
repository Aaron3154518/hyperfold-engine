// Discard a value
pub trait Discard {
    fn discard(self) -> ();
}

impl<T> Discard for T {
    fn discard(self) -> () {}
}

// Convert () to None
pub trait ToNone {
    fn none<T>(self) -> Option<T>;
}

impl ToNone for () {
    fn none<T>(self) -> Option<T> {
        None
    }
}
