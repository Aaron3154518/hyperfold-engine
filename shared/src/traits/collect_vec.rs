use std::{
    collections::{hash_map, HashMap},
    iter::Enumerate,
    slice::Iter,
    vec,
};

// Trait for operations on iterators that result in vecs
pub trait CollectVecInto<T, I: Iterator<Item = T>>
where
    Self: Sized,
{
    fn get_iter_into(self) -> I;

    fn enumer_iter_into(self) -> Enumerate<I> {
        self.get_iter_into().enumerate()
    }

    fn filter_vec_into<F>(self, f: F) -> Vec<T>
    where
        F: FnMut(&T) -> bool,
    {
        self.get_iter_into().filter(f).collect()
    }

    fn enumer_filter_vec_into<F>(self, f: F) -> Vec<T>
    where
        F: FnMut(&(usize, T)) -> bool,
    {
        self.enumer_iter_into().filter(f).map(|(_, t)| t).collect()
    }

    fn map_vec_into<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> U,
    {
        self.get_iter_into().map(f).collect()
    }

    fn enumer_map_vec_into<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut((usize, T)) -> U,
    {
        self.enumer_iter_into().map(f).collect()
    }

    fn filter_map_vec_into<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> Option<U>,
    {
        self.get_iter_into().filter_map(f).collect()
    }

    fn enumer_filter_map_vec_into<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut((usize, T)) -> Option<U>,
    {
        self.enumer_iter_into().filter_map(f).collect()
    }

    fn join_map_into<F>(self, f: F, sep: &str) -> String
    where
        F: FnMut(T) -> String,
    {
        self.get_iter_into().map_vec_into(f).join(sep)
    }

    fn unzip_vec_into<U, V, F>(self, f: F) -> (Vec<U>, Vec<V>)
    where
        F: FnMut(T) -> (U, V),
    {
        self.get_iter_into().map(f).unzip()
    }

    fn unzipn_vec_into<U, V, F>(self, f: F, g: impl FnOnce(std::iter::Map<I, F>) -> V) -> V
    where
        F: FnMut(T) -> U,
    {
        g(self.get_iter_into().map(f))
    }

    fn flatten_map_vec_into<U, V, F>(self, f: F) -> Vec<V>
    where
        T: Iterator<Item = U>,
        F: FnMut(U) -> V,
    {
        self.get_iter_into().flatten().map(f).collect()
    }
}

impl<T, I> CollectVecInto<T, Self> for I
where
    I: Iterator<Item = T>,
{
    fn get_iter_into(self) -> Self {
        self
    }
}

impl<T> CollectVecInto<T, vec::IntoIter<T>> for Vec<T> {
    fn get_iter_into(self) -> vec::IntoIter<T> {
        self.into_iter()
    }
}

impl<T, const N: usize> CollectVecInto<T, std::array::IntoIter<T, N>> for [T; N] {
    fn get_iter_into(self) -> std::array::IntoIter<T, N> {
        self.into_iter()
    }
}

impl<K, V> CollectVecInto<(K, V), hash_map::IntoIter<K, V>> for HashMap<K, V> {
    fn get_iter_into(self) -> hash_map::IntoIter<K, V> {
        self.into_iter()
    }
}

pub trait CollectVec<'a, T: 'a, I>
where
    I: CollectVecInto<T, I> + Iterator<Item = T>,
{
    fn get_iter(&'a self) -> I;

    fn enumer_iter(&'a self) -> Enumerate<I> {
        self.get_iter().enumerate()
    }

    fn map_vec<U, F>(&'a self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> U,
    {
        self.get_iter().map_vec_into(f)
    }

    fn enumer_map_vec<U, F>(&'a self, f: F) -> Vec<U>
    where
        F: FnMut((usize, T)) -> U,
    {
        self.enumer_iter().map_vec_into(f)
    }

    fn filter_map_vec<U, F>(&'a self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> Option<U>,
    {
        self.get_iter().filter_map_vec_into(f)
    }

    fn enumer_filter_map_vec<U, F>(&'a self, f: F) -> Vec<U>
    where
        F: FnMut((usize, T)) -> Option<U>,
    {
        self.enumer_iter().filter_map_vec_into(f)
    }

    fn join_map<F>(&'a self, f: F, sep: &str) -> String
    where
        F: FnMut(T) -> String,
    {
        self.map_vec(f).join(sep)
    }

    fn enumer_join_map<F>(&'a self, f: F, sep: &str) -> String
    where
        F: FnMut((usize, T)) -> String,
    {
        self.enumer_map_vec(f).join(sep)
    }

    fn unzip_vec<U, V, F>(&'a self, f: F) -> (Vec<U>, Vec<V>)
    where
        F: FnMut(T) -> (U, V),
    {
        self.get_iter().unzip_vec_into(f)
    }

    fn enumer_unzip_vec<U, V, F>(&'a self, f: F) -> (Vec<U>, Vec<V>)
    where
        F: FnMut((usize, T)) -> (U, V),
    {
        self.enumer_iter().unzip_vec_into(f)
    }

    fn unzipn_vec<U, V, F>(&'a self, f: F, g: impl FnOnce(std::iter::Map<I, F>) -> V) -> V
    where
        F: FnMut(T) -> U,
    {
        self.get_iter().unzipn_vec_into(f, g)
    }

    fn enumer_unzipn_vec<U, V, F>(
        &'a self,
        f: F,
        g: impl FnOnce(std::iter::Map<Enumerate<I>, F>) -> V,
    ) -> V
    where
        F: FnMut((usize, T)) -> U,
    {
        self.enumer_iter().unzipn_vec_into(f, g)
    }

    fn flatten_map_vec<U, V, F>(&'a self, f: F) -> Vec<V>
    where
        T: Iterator<Item = U>,
        F: FnMut(U) -> V,
    {
        self.get_iter().flatten_map_vec_into(f)
    }
}

impl<'a, T: 'a> CollectVec<'a, &'a T, Iter<'a, T>> for Vec<T> {
    fn get_iter(&'a self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, K: 'a, V: 'a> CollectVec<'a, (&'a K, &'a V), hash_map::Iter<'a, K, V>> for HashMap<K, V> {
    fn get_iter(&'a self) -> hash_map::Iter<'a, K, V> {
        self.iter()
    }
}

impl<'a, T: 'a, const N: usize> CollectVec<'a, &'a T, Iter<'a, T>> for [T; N] {
    fn get_iter(&'a self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T: 'a> CollectVec<'a, &'a T, Iter<'a, T>> for [T] {
    fn get_iter(&'a self) -> Iter<'a, T> {
        self.iter()
    }
}
