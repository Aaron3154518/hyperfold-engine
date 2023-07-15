#![feature(drain_filter)]
#![feature(hash_drain_filter)]
#![feature(array_methods)]
#![feature(iter_intersperse)]
#![feature(lazy_cell)]
#![allow(unused)]

mod codegen;
mod component_set;
mod parse;
mod resolve;
mod system;
mod utils;

use regex::Regex;
use resolve::ItemsCrate;
use std::{
    collections::HashMap,
    fs::{self, File},
    hash::Hash,
    io::Read,
    path::PathBuf,
};

use component_set::ComponentSetLabels;
use parse::{AstCrate, ComponentSymbol};
use shared::{macros::hash_map, parsing::ComponentMacroArgs, traits::CollectVecInto};
use utils::syn::format_code;

use crate::parse::resolve_path;

// Process:
// 1) Parse AST, get mod/crate structure, use statements, and important syntax items
// 2) Populate with macro symbols
// 3) Parse components, events, globals; insert symbols
// 4) Parse component sets; Validate labels; insert symbols
// 5) Parse systems; Validate arguments; insert symbols
// 6) Codegen

fn test_resolves(crates: &Vec<AstCrate>) {
    let test = |v: Vec<&str>| {
        println!(
            "{}\n{:#?}",
            v.join("::"),
            resolve_path(
                v.iter().map(|s| s.to_string()).collect(),
                (&crates[0].main, &crates[0], &crates)
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
    let mut crates = AstCrate::parse(PathBuf::from("../"));

    // eprintln!("{crates:#?}");

    let mut items = ItemsCrate::parse(&mut crates);

    // eprintln!("{items:#?}");
}
