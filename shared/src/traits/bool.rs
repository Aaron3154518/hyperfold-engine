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

// then_none for bool
pub trait ThenNone {
    fn then_none<T>(self, t: T) -> Option<T>;
}

impl ThenNone for bool {
    fn then_none<T>(self, t: T) -> Option<T> {
        match self {
            true => None,
            false => Some(t),
        }
    }
}
