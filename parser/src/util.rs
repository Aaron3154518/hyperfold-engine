extern crate alloc;

use std::{
    iter::{Enumerate, Map},
    path::PathBuf,
};

use regex::Regex;

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
}

impl<T> JoinMap<T> for Vec<T> {
    fn map_vec<U, F>(&self, f: F) -> Vec<U>
    where
        F: FnMut(&T) -> U,
    {
        self.iter().map(f).collect()
    }
}

impl<T, const N: usize> JoinMap<T> for [T; N] {
    fn map_vec<U, F>(&self, f: F) -> Vec<U>
    where
        F: FnMut(&T) -> U,
    {
        self.iter().map(f).collect()
    }
}

impl<T> JoinMap<T> for [T] {
    fn map_vec<U, F>(&self, f: F) -> Vec<U>
    where
        F: FnMut(&T) -> U,
    {
        self.iter().map(f).collect()
    }
}

pub trait JoinMapInto<T> {
    fn map_vec<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> U;

    fn join_map<F>(self, f: F, sep: &str) -> String
    where
        F: FnMut(T) -> String;
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

#[inline]
pub fn end<T>(v: &Vec<T>, off: usize) -> usize {
    v.len().max(off) - off
}

// Manage use statements
pub fn parse_vec_path(parent_path: &Vec<String>, path: &Vec<String>) -> Vec<String> {
    let mut res_path: Vec<String> = Vec::new();
    for p in path {
        match p.as_str() {
            "super" => {
                if res_path.is_empty() {
                    res_path = parent_path[..end(parent_path, 1)].to_vec();
                } else {
                    res_path.pop();
                }
            }
            _ => res_path.push(p.to_string()),
        }
    }
    res_path
}

pub fn parse_syn_path(parent_path: &Vec<String>, path: &syn::Path) -> Vec<String> {
    parse_vec_path(
        parent_path,
        &path.segments.iter().map(|s| s.ident.to_string()).collect(),
    )
}

// Minimal code formatting for token streams
pub fn format_code(s: String) -> String {
    let space_reg_l = Regex::new(r"(^|\w|\)) (:|::|<|>|;|\.|\(|,|&|})")
        .expect("Could not parse left space codegen regex");
    let space_reg_r = Regex::new(r"(::|<|;|\.|\)|&|\{|}) (\w|\(|$)")
        .expect("Could not parse right space codegen regex");
    brackets(
        space_reg_l
            .replace_all(
                space_reg_r
                    .replace_all(s.replace("; ", ";\n").as_str(), "${1}${2}")
                    .to_string()
                    .as_str(),
                "${1}${2}",
            )
            .to_string(),
    )
}

pub fn brackets(s: String) -> String {
    let mut l_is = s.match_indices("{");
    let mut r_is = s.match_indices("}");
    let mut l_i = l_is.next();
    let mut r_i = r_is.next();
    let idx1 = if let Some((i, _)) = l_i { i } else { return s };
    let mut cnt: usize = 0;
    while let Some((r, _)) = r_i {
        if l_i.is_some_and(|(l, _)| l <= r) {
            l_i = l_is.next();
            cnt += 1;
        } else {
            r_i = r_is.next();
            if cnt == 1 {
                let mid = brackets(s[idx1 + 1..r].to_string())
                    .split_collect::<Vec<_>>("\n")
                    .join("\n\t");
                return format!(
                    "{}{{{}}}{}{}",
                    s[..idx1].to_string(),
                    if mid.trim().is_empty() {
                        String::new()
                    } else {
                        format!("\n\t{}\n", mid)
                    },
                    if r_i.is_some_and(|(r2, _)| r2 != r + 1) {
                        "\n"
                    } else {
                        ""
                    },
                    brackets(s[r + 1..].to_string())
                );
            } else if cnt > 0 {
                cnt -= 1;
            }
        }
    }
    s
}
