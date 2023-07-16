use std::ops::Range;

pub trait RangeTrait {
    fn add_into(self, x: usize) -> Self;

    fn add(&mut self, x: usize);

    fn sub_into(self, x: usize) -> Self;

    fn sub(&mut self, x: usize);
}

impl RangeTrait for Range<usize> {
    fn add_into(self, x: usize) -> Self {
        self.start + x..self.end + x
    }

    fn add(&mut self, x: usize) {
        self.start += x;
        self.end += x;
    }

    fn sub_into(self, x: usize) -> Self {
        self.start - x..self.end - x
    }

    fn sub(&mut self, x: usize) {
        self.start += x;
        self.end += x;
    }
}
