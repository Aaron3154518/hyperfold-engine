use std::{
    collections::HashMap,
    iter::Enumerate,
    ops::{Add, AddAssign},
    slice::Iter,
    str::pattern::Pattern,
    vec::IntoIter,
};

extern crate alloc;

// Prefix/postfix ++
pub trait Increment
where
    Self: Copy,
{
    fn add_then(&mut self, v: Self) -> Self;

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

// Traits for calling except() with a String (i.e. with format!())
pub trait Catch<T> {
    fn catch(self, err: String) -> T;
}

impl<T> Catch<T> for Option<T> {
    fn catch(self, err: String) -> T {
        self.expect(err.as_str())
    }
}

impl<T, E> Catch<T> for Result<T, E>
where
    E: std::fmt::Debug,
{
    fn catch(self, err: String) -> T {
        self.expect(err.as_str())
    }
}

// Trait for getting the value of a Result regardless of Ok/Err
pub trait Get<T> {
    fn get(self) -> T;
}

impl<T> Get<T> for Result<T, T> {
    fn get(self) -> T {
        match self {
            Ok(t) | Err(t) => t,
        }
    }
}

// Generalized unzip
macro_rules! unzip {
    ((), ($($vs: ident: $ts: ident),*)) => {};

    (($f: ident: $tr: ident $(,$fs: ident: $trs: ident)*), ($v: ident: $t: ident $(,$vs: ident: $ts: ident)*)) => {
        unzip!(($($fs: $trs),*), ($($vs: $ts),*));

        pub trait $tr<$t $(,$ts)*> {
            fn $f(self) -> (Vec<$t> $(,Vec<$ts>)*);
        }

        impl<$t $(,$ts)*> $tr<$t $(,$ts)*> for alloc::vec::IntoIter<($t $(,$ts)*)> {
            fn $f(self) -> (Vec<$t> $(,Vec<$ts>)*) {
                self.fold(
                    (Vec::<$t>::new() $(, Vec::<$ts>::new())*),
                    #[allow(non_snake_case)]
                    |(mut $t $(,mut $ts)*), ($v $(,$vs)*)| {
                        $t.push($v);
                        $($ts.push($vs);)*
                        ($t $(,$ts)*)
                    }
                )
            }
        }
    };
}

unzip!(
    (
        unzip8_vec: Unzip8,
        unzip7_vec: Unzip7,
        unzip6_vec: Unzip6,
        unzip5_vec: Unzip5,
        unzip4_vec: Unzip4,
        unzip3_vec: Unzip3
    ),
    (a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H)
);

// Trait for mapping Vec elements to strings and joining them
pub trait JoinMapInto<T, I: Iterator<Item = T>>
where
    Self: Sized,
{
    fn get_iter_into(self) -> I;

    fn filter_vec_into<F>(self, f: F) -> Vec<T>
    where
        F: FnMut(&T) -> bool,
    {
        self.get_iter_into().filter(f).collect()
    }

    fn map_vec_into<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> U,
    {
        self.get_iter_into().map(f).collect()
    }

    fn filter_map_vec_into<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> Option<U>,
    {
        self.get_iter_into().filter_map(f).collect()
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
}

impl<T, I> JoinMapInto<T, Self> for I
where
    I: Iterator<Item = T>,
{
    fn get_iter_into(self) -> Self {
        self
    }
}

impl<T> JoinMapInto<T, IntoIter<T>> for Vec<T> {
    fn get_iter_into(self) -> IntoIter<T> {
        self.into_iter()
    }
}

pub trait JoinMap<'a, T: 'a> {
    fn get_iter(&'a self) -> Iter<'a, T>;

    fn get_enumerate(&'a self) -> Enumerate<Iter<'a, T>> {
        self.get_iter().enumerate()
    }

    fn map_vec<U, F>(&'a self, f: F) -> Vec<U>
    where
        F: FnMut(&'a T) -> U,
    {
        self.get_iter().map_vec_into(f)
    }

    fn enumerate_map_vec<U, F>(&'a self, f: F) -> Vec<U>
    where
        F: FnMut((usize, &'a T)) -> U,
    {
        self.get_enumerate().map_vec_into(f)
    }

    fn filter_map_vec<U, F>(&'a self, f: F) -> Vec<U>
    where
        F: FnMut(&'a T) -> Option<U>,
    {
        self.get_iter().filter_map_vec_into(f)
    }

    fn enumerate_filter_map_vec<U, F>(&'a self, f: F) -> Vec<U>
    where
        F: FnMut((usize, &'a T)) -> Option<U>,
    {
        self.get_enumerate().filter_map_vec_into(f)
    }

    fn join_map<F>(&'a self, f: F, sep: &str) -> String
    where
        F: FnMut(&'a T) -> String,
    {
        self.map_vec(f).join(sep)
    }

    fn enumerate_join_map<F>(&'a self, f: F, sep: &str) -> String
    where
        F: FnMut((usize, &'a T)) -> String,
    {
        self.enumerate_map_vec(f).join(sep)
    }

    fn unzip_vec<U, V, F>(&'a self, f: F) -> (Vec<U>, Vec<V>)
    where
        F: FnMut(&'a T) -> (U, V),
    {
        self.get_iter().unzip_vec_into(f)
    }

    fn enumerate_unzip_vec<U, V, F>(&'a self, f: F) -> (Vec<U>, Vec<V>)
    where
        F: FnMut((usize, &'a T)) -> (U, V),
    {
        self.get_enumerate().unzip_vec_into(f)
    }
}

impl<'a, T: 'a> JoinMap<'a, T> for Vec<T> {
    fn get_iter(&'a self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T: 'a, const N: usize> JoinMap<'a, T> for [T; N] {
    fn get_iter(&'a self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T: 'a> JoinMap<'a, T> for [T] {
    fn get_iter(&'a self) -> Iter<'a, T> {
        self.iter()
    }
}

// Trait for flattening and mapping
pub trait FlattenMap<'a, T> {
    fn flatten_map_vec<U, F>(&'a self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> U;
}

impl<'a, T: 'a> FlattenMap<'a, &'a T> for Vec<Vec<T>> {
    fn flatten_map_vec<U, F>(&'a self, f: F) -> Vec<U>
    where
        F: FnMut(&'a T) -> U,
    {
        self.iter().flatten().map(f).collect()
    }
}

pub trait FlattenMapInto<T> {
    fn flatten_map_vec_into<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> U;
}

impl<T> FlattenMapInto<T> for Vec<Vec<T>> {
    fn flatten_map_vec_into<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> U,
    {
        self.into_iter().flatten().map(f).collect()
    }
}

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
    fn map_none(self, f: impl FnOnce() -> T) -> Option<T>;
}

impl<T, U> MapNone<T> for Option<U> {
    fn map_none(self, f: impl FnOnce() -> T) -> Option<T> {
        match self {
            Some(_) => None,
            None => Some(f()),
        }
    }
}

// Trait for appling a function to a type as a member function
// Used for splitting tuples
pub trait Call<T, V> {
    fn call(&self, f: impl FnOnce(&T) -> V) -> V;

    fn call_into(self, f: impl FnOnce(T) -> V) -> V;
}

impl<T, V> Call<Self, V> for T {
    fn call(&self, f: impl FnOnce(&Self) -> V) -> V {
        f(&self)
    }

    fn call_into(self, f: impl FnOnce(Self) -> V) -> V {
        f(self)
    }
}

// Splitting string into list
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

// Split a vector around an element
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

// Search from position
pub trait FindFrom<'a, P> {
    fn length(&self) -> usize;

    fn find_from(&'a self, p: P, pos: usize) -> Option<usize> {
        self.find_from_to(p, pos, self.length())
    }

    fn find_to(&'a self, p: P, pos: usize) -> Option<usize> {
        self.find_from_to(p, 0, pos)
    }

    fn find_from_to(&'a self, p: P, pos1: usize, pos2: usize) -> Option<usize>;
}

impl<'a, P> FindFrom<'a, P> for String
where
    P: Pattern<'a>,
{
    fn length(&self) -> usize {
        self.len()
    }

    fn find_from_to(&'a self, pat: P, pos1: usize, pos2: usize) -> Option<usize> {
        self[pos1..pos2].find(pat).map(|idx| idx + pos1)
    }
}

impl<'a, P> FindFrom<'a, P> for &str
where
    P: Pattern<'a>,
{
    fn length(&self) -> usize {
        self.len()
    }

    fn find_from_to(&'a self, pat: P, pos1: usize, pos2: usize) -> Option<usize> {
        self[pos1..pos2].find(pat).map(|idx| idx + pos1)
    }
}

impl<'a, F, T> FindFrom<'a, F> for Vec<T>
where
    T: 'a,
    F: Fn(&'a T) -> bool,
{
    fn length(&self) -> usize {
        self.len()
    }

    fn find_from_to(&'a self, f: F, pos1: usize, pos2: usize) -> Option<usize> {
        self[pos1..pos2].iter().position(f).map(|idx| idx + pos1)
    }
}

// Add element to vec in place
pub trait PushInto<T> {
    fn push_into(self, t: T) -> Self;
}

impl<T> PushInto<T> for Vec<T> {
    fn push_into(mut self, t: T) -> Self {
        self.push(t);
        self
    }
}

// then_some for Result
pub trait ThenOk<T, E> {
    fn ok(&self, t: T, e: E) -> Result<T, E>;

    fn then_ok<Ft, Fe>(&self, t: Ft, e: Fe) -> Result<T, E>
    where
        Ft: FnOnce() -> T,
        Fe: FnOnce() -> E;

    fn err(&self, e: E, t: T) -> Result<T, E>;

    fn then_err<Ft, Fe>(&self, e: Fe, t: Ft) -> Result<T, E>
    where
        Ft: FnOnce() -> T,
        Fe: FnOnce() -> E;
}

impl<T, E> ThenOk<T, E> for bool {
    fn ok(&self, t: T, e: E) -> Result<T, E> {
        match self {
            true => Ok(t),
            false => Err(e),
        }
    }

    fn then_ok<Ft, Fe>(&self, t: Ft, e: Fe) -> Result<T, E>
    where
        Ft: FnOnce() -> T,
        Fe: FnOnce() -> E,
    {
        match self {
            true => Ok(t()),
            false => Err(e()),
        }
    }

    fn err(&self, e: E, t: T) -> Result<T, E> {
        match self {
            true => Err(e),
            false => Ok(t),
        }
    }

    fn then_err<Ft, Fe>(&self, e: Fe, t: Ft) -> Result<T, E>
    where
        Ft: FnOnce() -> T,
        Fe: FnOnce() -> E,
    {
        match self {
            true => Err(e()),
            false => Ok(t()),
        }
    }
}

// ok() for Result but handle the error
pub trait HandleErr<T, E> {
    fn handle_err<F>(self, f: F) -> Option<T>
    where
        F: FnOnce(E);
}

impl<T, E> HandleErr<T, E> for Result<T, E> {
    fn handle_err<F>(self, f: F) -> Option<T>
    where
        F: FnOnce(E),
    {
        match self {
            Ok(t) => Some(t),
            Err(e) => {
                f(e);
                None
            }
        }
    }
}

// Get for multidimensional vecetor
pub trait Get2D<T> {
    fn get<'a>(&'a self, i: usize, j: usize) -> Option<&'a T>;

    fn get_mut<'a>(&'a mut self, i: usize, j: usize) -> Option<&'a mut T>;
}

impl<T> Get2D<T> for Vec<Vec<T>> {
    fn get<'a>(&'a self, i: usize, j: usize) -> Option<&'a T> {
        <[Vec<T>]>::get(self, i).and_then(|v| v.get(j))
    }

    fn get_mut<'a>(&'a mut self, i: usize, j: usize) -> Option<&'a mut T> {
        <[Vec<T>]>::get_mut(self, i).and_then(|v| v.get_mut(j))
    }
}

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
