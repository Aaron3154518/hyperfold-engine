use std::{
    iter::Enumerate,
    ops::{Add, AddAssign},
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

        // impl<$t $(,$ts)*> $tr<$t $(,$ts)*> for alloc::vec::IntoIter<($t $(,$ts)*)> {
        //     fn unzip_vec(self) -> (Vec<$t> $(,Vec<$ts>)*) {

        //     }
        // }
    };
}

unzip!(
    (
        unzip7_vec: Unzip7,
        unzip6_vec: Unzip6,
        unzip5_vec: Unzip5,
        unzip4_vec: Unzip4,
        unzip3_vec: Unzip3
    ),
    (a: A, b: B, c: C, d: D, e: E, f: F, g: G)
);

// Trait for mapping Vec elements to strings and joining them
pub trait JoinMap<T> {
    fn map_vec<U, F>(&self, f: F) -> Vec<U>
    where
        F: FnMut(&T) -> U;

    fn join_map<F>(&self, f: F, sep: &str) -> String
    where
        F: FnMut(&T) -> String,
    {
        self.map_vec(f).join(sep)
    }

    fn unzip_vec<U, V, F>(&self, f: F) -> (Vec<U>, Vec<V>)
    where
        F: FnMut(&T) -> (U, V);
}

impl<T> JoinMap<T> for Vec<T> {
    fn map_vec<U, F>(&self, f: F) -> Vec<U>
    where
        F: FnMut(&T) -> U,
    {
        self.iter().map(f).collect()
    }

    fn unzip_vec<U, V, F>(&self, f: F) -> (Vec<U>, Vec<V>)
    where
        F: FnMut(&T) -> (U, V),
    {
        self.iter().map(f).unzip()
    }
}

impl<T, const N: usize> JoinMap<T> for [T; N] {
    fn map_vec<U, F>(&self, f: F) -> Vec<U>
    where
        F: FnMut(&T) -> U,
    {
        self.iter().map(f).collect()
    }

    fn unzip_vec<U, V, F>(&self, f: F) -> (Vec<U>, Vec<V>)
    where
        F: FnMut(&T) -> (U, V),
    {
        self.iter().map(f).unzip()
    }
}

impl<T> JoinMap<T> for [T] {
    fn map_vec<U, F>(&self, f: F) -> Vec<U>
    where
        F: FnMut(&T) -> U,
    {
        self.iter().map(f).collect()
    }

    fn unzip_vec<U, V, F>(&self, f: F) -> (Vec<U>, Vec<V>)
    where
        F: FnMut(&T) -> (U, V),
    {
        self.iter().map(f).unzip()
    }
}

pub trait JoinMapInto<T> {
    fn map_vec<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> U;

    fn join_map<F>(self, f: F, sep: &str) -> String
    where
        F: FnMut(T) -> String;

    fn unzip_vec<U, V, F>(self, f: F) -> (Vec<U>, Vec<V>)
    where
        F: FnMut(T) -> (U, V);
}

impl<'a, T> JoinMapInto<&'a T> for core::slice::Iter<'a, T> {
    fn map_vec<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut(&'a T) -> U,
    {
        self.map(f).collect()
    }

    fn join_map<F>(self, f: F, sep: &str) -> String
    where
        F: FnMut(&'a T) -> String,
    {
        self.map_vec(f).join(sep)
    }

    fn unzip_vec<U, V, F>(self, f: F) -> (Vec<U>, Vec<V>)
    where
        F: FnMut(&'a T) -> (U, V),
    {
        self.map(f).unzip()
    }
}

impl<T> JoinMapInto<T> for alloc::vec::IntoIter<T> {
    fn map_vec<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> U,
    {
        self.map(f).collect()
    }

    fn join_map<F>(self, f: F, sep: &str) -> String
    where
        F: FnMut(T) -> String,
    {
        self.map_vec(f).join(sep)
    }

    fn unzip_vec<U, V, F>(self, f: F) -> (Vec<U>, Vec<V>)
    where
        F: FnMut(T) -> (U, V),
    {
        self.map(f).unzip()
    }
}

impl<T, Iter: Iterator<Item = T>> JoinMapInto<(usize, T)> for Enumerate<Iter> {
    fn map_vec<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut((usize, T)) -> U,
    {
        self.map(f).collect()
    }

    fn join_map<F>(self, f: F, sep: &str) -> String
    where
        F: FnMut((usize, T)) -> String,
    {
        self.map_vec(f).join(sep)
    }

    fn unzip_vec<U, V, F>(self, f: F) -> (Vec<U>, Vec<V>)
    where
        F: FnMut((usize, T)) -> (U, V),
    {
        self.map(f).unzip()
    }
}

impl<'a> JoinMapInto<&'a str> for std::str::Split<'a, &str> {
    fn map_vec<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut(&'a str) -> U,
    {
        self.map(f).collect()
    }

    fn join_map<F>(self, f: F, sep: &str) -> String
    where
        F: FnMut(&'a str) -> String,
    {
        self.map_vec(f).join(sep)
    }

    fn unzip_vec<U, V, F>(self, f: F) -> (Vec<U>, Vec<V>)
    where
        F: FnMut(&'a str) -> (U, V),
    {
        self.map(f).unzip()
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

// Flatten 2D -> 1D
pub trait Flatten<'a, T>
where
    T: 'a,
{
    fn flatten<V>(self, v: V) -> V
    where
        V: Extend<&'a T>;
}

impl<'a, A, B, T> Flatten<'a, T> for A
where
    A: IntoIterator<Item = B>,
    B: IntoIterator<Item = &'a T>,
    T: 'a,
{
    fn flatten<V>(self, v: V) -> V
    where
        V: Extend<&'a T>,
    {
        self.into_iter().fold(v, |mut v, t| {
            v.extend(t.into_iter());
            v
        })
    }
}
