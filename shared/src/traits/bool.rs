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

// simplified match statements
pub trait MapOr {
    fn map_or<T>(self, t: T, f: T) -> T;

    fn then_or<T>(self, t: impl FnOnce() -> T, f: impl FnOnce() -> T) -> T;
}

impl MapOr for bool {
    fn map_or<T>(self, t: T, f: T) -> T {
        match self {
            true => t,
            false => f,
        }
    }

    fn then_or<T>(self, t: impl FnOnce() -> T, f: impl FnOnce() -> T) -> T {
        match self {
            true => t(),
            false => f(),
        }
    }
}
