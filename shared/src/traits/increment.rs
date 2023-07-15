use std::ops::{Add, AddAssign};

// Prefix/postfix ++
pub trait Increment
where
    Self: Copy,
{
    // ++i
    fn add_then(&mut self, v: Self) -> Self;

    // i++
    fn then_add(&mut self, v: Self) -> Self {
        let s = *self;
        self.add_then(v);
        s
    }
}

impl<T> Increment for T
where
    T: Copy + Add + AddAssign,
{
    fn add_then(&mut self, v: Self) -> Self {
        *self += v;
        *self
    }
}
