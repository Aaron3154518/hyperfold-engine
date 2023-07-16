use std::vec;

// Get for multidimensional vecetor
pub trait Get2D<T> {
    fn get2d<'a>(&'a self, i: usize, j: usize) -> Option<&'a T>;

    fn get2d_mut<'a>(&'a mut self, i: usize, j: usize) -> Option<&'a mut T>;
}

impl<T> Get2D<T> for Vec<Vec<T>> {
    fn get2d<'a>(&'a self, i: usize, j: usize) -> Option<&'a T> {
        <[Vec<T>]>::get(self, i).and_then(|v| v.get(j))
    }

    fn get2d_mut<'a>(&'a mut self, i: usize, j: usize) -> Option<&'a mut T> {
        <[Vec<T>]>::get_mut(self, i).and_then(|v| v.get_mut(j))
    }
}

// Get first into
pub trait GetFirst<T, I>
where
    I: Iterator<Item = T>,
{
    fn first_into(self) -> Option<(T, I)>;
}

impl<T> GetFirst<T, vec::IntoIter<T>> for Vec<T> {
    fn first_into(self) -> Option<(T, vec::IntoIter<T>)> {
        let mut it = self.into_iter();
        it.next().map(|t| (t, it))
    }
}

// Get offset from end of a vector (replicate e.g. arr[:-1] in python)
pub trait End {
    fn end(&self, off: usize) -> usize;
}

impl<T> End for Vec<T> {
    #[inline]
    fn end(&self, off: usize) -> usize {
        self.len().max(off) - off
    }
}

// Get range, supports python indexes
pub trait GetSlice<T> {
    fn get_len(&self) -> usize;

    fn slice<'a>(&'a self, s: isize, e: isize) -> &'a [T];

    fn slice_from<'a>(&'a self, s: isize) -> &'a [T] {
        self.slice(s, self.get_len() as isize)
    }

    fn slice_to<'a>(&'a self, e: isize) -> &'a [T] {
        self.slice(0, e)
    }
}

impl<T> GetSlice<T> for Vec<T> {
    fn get_len(&self) -> usize {
        self.len()
    }

    fn slice<'a>(&'a self, s: isize, e: isize) -> &'a [T] {
        let (neg, s) = (s < 0, s.unsigned_abs());
        let s = if neg { self.end(s) } else { s };
        let (neg, e) = (e < 0, e.unsigned_abs());
        let e = if neg { self.end(e) } else { e };
        &self[s..e]
    }
}
