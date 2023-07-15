use super::Call;

// Split string into vec
pub trait SplitCollect {
    fn split_collect<V>(&self, sep: &str) -> V
    where
        V: FromIterator<String>;

    fn split_map<F, T, V>(&self, sep: &str, f: F) -> V
    where
        F: FnMut(&str) -> T,
        V: FromIterator<T>;
}

impl SplitCollect for String {
    fn split_collect<V>(&self, sep: &str) -> V
    where
        V: FromIterator<String>,
    {
        self.split(sep).map(|s| s.to_string()).collect()
    }

    fn split_map<F, T, V>(&self, sep: &str, f: F) -> V
    where
        F: FnMut(&str) -> T,
        V: FromIterator<T>,
    {
        self.split(sep).map(f).collect()
    }
}

impl SplitCollect for str {
    fn split_collect<V>(&self, sep: &str) -> V
    where
        V: FromIterator<String>,
    {
        self.split(sep).map(|s| s.to_string()).collect()
    }

    fn split_map<F, T, V>(&self, sep: &str, f: F) -> V
    where
        F: FnMut(&str) -> T,
        V: FromIterator<T>,
    {
        self.split(sep).map(f).collect()
    }
}

// Split vector around an element
pub trait SplitAround<T> {
    fn split_around<'a>(&'a self, i: usize) -> (&'a [T], &'a T, &'a [T]);

    fn split_around_mut<'a>(&'a mut self, i: usize) -> (&'a mut [T], &'a mut T, &'a mut [T]);
}

pub trait SplitAroundCopy<T> {
    fn split_around_copy(&self, i: usize) -> (Vec<T>, T, Vec<T>);
}

impl<T> SplitAround<T> for Vec<T> {
    fn split_around<'a>(&'a self, i: usize) -> (&'a [T], &'a T, &'a [T]) {
        self.split_at(i).call_into(|(left, mid_right)| {
            mid_right
                .split_at(1)
                .call_into(|(mid, right)| (left, &mid[0], right))
        })
    }

    fn split_around_mut<'a>(&'a mut self, i: usize) -> (&'a mut [T], &'a mut T, &'a mut [T]) {
        self.split_at_mut(i).call_into(|(left, mid_right)| {
            mid_right
                .split_at_mut(1)
                .call_into(|(mid, right)| (left, &mut mid[0], right))
        })
    }
}

impl<T> SplitAroundCopy<T> for Vec<T>
where
    T: Clone,
{
    fn split_around_copy(&self, i: usize) -> (Vec<T>, T, Vec<T>) {
        (self[..i].to_vec(), self[i].clone(), self[i + 1..].to_vec())
    }
}
