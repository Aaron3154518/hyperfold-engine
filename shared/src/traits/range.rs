use std::ops::Range;

pub trait ShiftRange {
    fn add(self, x: usize) -> Self;

    fn sub(self, x: usize) -> Self;
}

impl ShiftRange for Range<usize> {
    fn add(self, x: usize) -> Self {
        self.start + x..self.end + x
    }

    fn sub(self, x: usize) -> Self {
        self.start - x..self.end - x
    }
}
