#![feature(drain_filter)]
#![feature(hash_drain_filter)]
#![feature(array_methods)]
#![allow(unused)]

use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use parse::ast_crate::Crate;
use regex::Regex;
use resolve::ast_resolve;
use shared::util::JoinMapInto;

use crate::{
    codegen::codegen::Decoder,
    resolve::{ast_items::ItemsCrate, ast_paths::Paths},
    util::{end, format_code},
    validate::ast_validate::ItemData,
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

fn test_resolves(crates: &Vec<Crate>) {
    let test = |v: Vec<&str>| {
        println!(
            "{}\n{:#?}",
            v.join("::"),
            ast_resolve::resolve(
                v.iter().map(|s| s.to_string()).collect(),
                &crates[0],
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
    let (crates, paths) = Crate::parse(PathBuf::from("test/a"));
    println!(
        "{:#?}",
        crates
            .iter()
            .enumerate()
            .map_vec(|(i, cr)| format!("{}: {}", i, cr.name))
    );
    // println!("{:#?}", crates);

    // test_resolves(&crates);

    // Skip macros crate for resolution phase
    let mut items = crates[..end(&crates, 1)]
        .iter()
        .map(|cr| {
            let mut ic = ItemsCrate::new();
            ic.parse_crate(cr, &paths, &crates);
            // Remove macros crate as crate dependency
            if let Some(i) = ic
                .dependencies
                .iter()
                .position(|d| d.cr_idx == crates.len() - 1)
            {
                ic.dependencies.swap_remove(i);
            }
            ic
        })
        .collect::<Vec<_>>();
    // println!("{:#?}", items);

    let data = ItemData::validate(&paths, &mut items);
    data.write_to_file();
    println!("{:#?}", data);

    let decoder = Decoder::new();
    // println!("{:#?}", decoder);
    println!(
        "{}",
        format_code(decoder.codegen(PathBuf::from("./test/a")).1.to_string())
    );
}
