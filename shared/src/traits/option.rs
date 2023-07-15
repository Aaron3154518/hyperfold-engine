// Trait for logic on None values in Options
pub trait NoneOr<T> {
    fn is_none_or_into(self, f: impl FnOnce(T) -> bool) -> bool;

    fn is_none_or(&self, f: impl FnOnce(&T) -> bool) -> bool;
}

impl<T> NoneOr<T> for Option<T> {
    fn is_none_or_into(self, f: impl FnOnce(T) -> bool) -> bool {
        !self.is_some_and(|t| !f(t))
    }

    fn is_none_or(&self, f: impl FnOnce(&T) -> bool) -> bool {
        match self {
            Some(t) => f(t),
            None => true,
        }
    }
}

// Trait for mapping None to a value and Some to None
pub trait MapNone<T> {
    fn then_map_none(self, f: impl FnOnce() -> T) -> Option<T>;

    fn map_none(self, t: T) -> Option<T>;
}

impl<T, U> MapNone<T> for Option<U> {
    fn then_map_none(self, f: impl FnOnce() -> T) -> Option<T> {
        match self {
            Some(_) => None,
            None => Some(f()),
        }
    }

    fn map_none(self, t: T) -> Option<T> {
        match self {
            Some(_) => None,
            None => Some(t),
        }
    }
}
