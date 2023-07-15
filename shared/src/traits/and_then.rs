// and_then for bool
pub trait AndThen {
    fn and_then<T, F>(self, f: F) -> Option<T>
    where
        F: FnOnce() -> Option<T>;
}

impl AndThen for bool {
    fn and_then<T, F>(self, f: F) -> Option<T>
    where
        F: FnOnce() -> Option<T>,
    {
        self.then_some(()).and_then(|_| f())
    }
}
