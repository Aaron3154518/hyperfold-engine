#![feature(drain_filter)]
#![feature(hash_drain_filter)]
#![feature(array_methods)]
#![allow(unused)]

use std::{
    collections::HashMap,
    fs::{self, File},
    hash::Hash,
    io::Read,
    path::PathBuf,
};

use parse::AstCrate;
use regex::Regex;
use resolve::{resolve_path, ItemsCrate, LabelItem, LabelOp, MustBe};
use shared::{hash_map, util::JoinMapInto};
use util::{end, format_code};

// pub mod codegen;
pub mod codegen2;
pub mod parse;
pub mod resolve;
pub mod util;
// pub mod validate;

// Process:
// 1) Parse AST, get mod/crate structure, use statements, and important syntax items
// 2) Populate with macro symbols
// 3) Parse components, events, globals; insert symbols
// 4) Parse component sets; Validate labels; insert symbols
// 5) Parse systems; Validate arguments; insert symbols
// 6) Codegen

fn labels_eq(labels: &LabelItem, truth: HashMap<usize, bool>, expected: HashMap<usize, MustBe>) {
    let given_str = format!("Given: {truth:#?}");
    let actual = labels.get_symbols(truth);
    match actual == expected {
        true => eprintln!("Passed: {labels}\n{given_str}"),
        false => {
            eprintln!("Failed: {labels}\n{given_str}\nExpected: {expected:#?}\nActual: {actual:#?}")
        }
    }
}

pub fn test_labels() {
    // (1 && (0 || !1 || !0))
    let labels = LabelItem::Expression {
        op: LabelOp::And,
        items: vec![
            LabelItem::Item {
                not: false,
                c_idx: 1,
            },
            LabelItem::Expression {
                op: LabelOp::Or,
                items: vec![
                    LabelItem::Item {
                        not: false,
                        c_idx: 0,
                    },
                    LabelItem::Item {
                        not: true,
                        c_idx: 1,
                    },
                    LabelItem::Item {
                        not: true,
                        c_idx: 0,
                    },
                ],
            },
        ],
    };
    labels_eq(
        &labels,
        HashMap::new(),
        hash_map!({
            0 => MustBe::Unknown,
            1 => MustBe::Value(true)
        }),
    );
    labels_eq(
        &labels,
        hash_map!({
            0 => true
        }),
        hash_map!({
            0 => MustBe::Value(true),
            1 => MustBe::Value(true)
        }),
    )
}

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
