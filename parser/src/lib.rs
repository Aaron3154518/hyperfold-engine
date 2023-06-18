#![feature(drain_filter)]
#![feature(hash_drain_filter)]
#![feature(array_methods)]
#![allow(unused)]

use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use parse::AstCrate;
use regex::Regex;
use resolve::path;
use shared::util::JoinMapInto;

use crate::{
    resolve::{items_crate::ItemsCrate, paths::Paths},
    util::{end, format_code},
};

pub mod codegen;
pub mod parse;
pub mod resolve;
pub mod util;
pub mod validate;

// Process:
// 1) Parse - for each crate: traverse AST, extract important items/uses/mods/dependencies
// 2) Resolve - resolve item paths within/across crates
// 3) Validate - convert IR to data format and validate items
// 4) Decode - convert data back to IR

fn test_resolves(crates: &Vec<AstCrate>) {
    let test = |v: Vec<&str>| {
        println!(
            "{}\n{:#?}",
            v.join("::"),
            path::resolve_path(
                v.iter().map(|s| s.to_string()).collect(),
                &crates[0],
                &crates[0].main,
                &crates
            )
        )
    };

    println!("\nOk:\n");
    for v in [
        vec!["crate", "T1"],
        vec!["crate", "a1", "A"],
        vec!["crate", "a2", "a5", "HEY"],
        vec!["crate", "a22", "a5", "HEY"],
        vec!["crate", "a22", "a2", "a5", "HEY"],
        vec!["crate", "a2", "a3", "A", "A1"],
        vec!["crate", "a2", "a3", "A", "A2"],
        vec!["crate", "a2", "a3", "B", "A2"],
        vec!["crate", "a2", "a3", "A3", "A1"],
        vec!["crate", "a2", "a2", "A3", "A1"],
        vec!["crate", "c", "e", "DC"],
    ] {
        test(v)
    }

    println!("\nErr:\n");
    for v in [
        vec!["engine", "component"],
        vec!["crate", "component"],
        vec!["crate", "a2", "a3", "mac", "global"],
    ] {
        test(v)
    }
}

fn test() {
    // TODO: hardcoded
    let (mut crates, paths) = AstCrate::parse(PathBuf::from("../"));

    // eprintln!("{crates:#?}");

    let mut items = ItemsCrate::parse(&paths, &mut crates);

    // eprintln!("{items:#?}");
}
