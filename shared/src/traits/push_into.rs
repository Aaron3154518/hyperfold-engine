use std::{collections::HashSet, hash::Hash, path::PathBuf};

// Add element to vec in place
pub trait PushInto<T> {
    fn push_into(self, t: T) -> Self;

    fn push_item(&mut self, t: T) -> &mut Self;

    fn pop_into(self) -> Self;

    fn pop_item(&mut self) -> &mut Self;
}

impl<T> PushInto<T> for Vec<T> {
    fn push_into(mut self, t: T) -> Self {
        self.push(t);
        self
    }

    fn push_item(&mut self, t: T) -> &mut Self {
        self.push(t);
        self
    }

    fn pop_into(mut self) -> Self {
        self.pop();
        self
    }

    fn pop_item(&mut self) -> &mut Self {
        self.pop();
        self
    }
}

impl PushInto<String> for PathBuf {
    fn push_into(mut self, t: String) -> Self {
        self.push(t);
        self
    }

    fn push_item(&mut self, t: String) -> &mut Self {
        self.push(t);
        self
    }

    fn pop_into(mut self) -> Self {
        self.pop();
        self
    }

    fn pop_item(&mut self) -> &mut Self {
        self.pop();
        self
    }
}

// Extend in place
pub trait ExtendInto<T> {
    fn extend_into(self, t: impl IntoIterator<Item = T>) -> Self;
}

impl<T> ExtendInto<T> for Vec<T> {
    fn extend_into(mut self, t: impl IntoIterator<Item = T>) -> Self {
        self.extend(t);
        self
    }
}

impl<T> ExtendInto<T> for HashSet<T>
where
    T: Eq + Hash,
{
    fn extend_into(mut self, t: impl IntoIterator<Item = T>) -> Self {
        self.extend(t);
        self
    }
}
