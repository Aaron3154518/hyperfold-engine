#![feature(drain_filter)]
#![feature(hash_drain_filter)]
#![feature(array_methods)]
#![feature(iter_intersperse)]
#![feature(lazy_cell)]
#![allow(unused)]

use std::{
    collections::HashMap,
    fs::{self, File},
    hash::Hash,
    io::Read,
    path::PathBuf,
};

use parse::{AstCrate, ComponentSymbol};
use regex::Regex;
use resolve::{
    resolve_path, ComponentSetLabels, ItemsCrate, LabelItem, LabelOp, LabelsExpression, MustBe,
};
use shared::{hash_map, parse_args::ComponentMacroArgs, util::JoinMapInto};
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

fn labels_const(labels: &mut LabelItem, truth: HashMap<ComponentSymbol, bool>, expected: bool) {
    let given_str = format!("Given: {truth:#?}");
    let actual = labels.evaluate_labels(truth);
    match actual {
        ComponentSetLabels::Constant(v) if v == expected => {
            eprintln!("Passed: {labels}\n{given_str}\n")
        }
        _ => {
            eprintln!("Failed: {labels}\n{given_str}\nExpected: {expected}\nActual: {actual:#?}\n")
        }
    }
}

fn labels_eq(
    labels: &mut LabelItem,
    truth: HashMap<ComponentSymbol, bool>,
    expected_labels: String,
    expected_truths: Vec<ComponentSymbol>,
    expected_falses: Vec<ComponentSymbol>,
    expected_unknowns: Vec<ComponentSymbol>,
) {
    let given_str = format!("Given: {truth:#?}");
    let actual = labels.evaluate_labels(truth);
    let mut msgs = Vec::new();
    match actual {
        ComponentSetLabels::Constant(v) => {
            msgs.push(format!("Expected: {expected_labels}\nActual: {v}"))
        }
        ComponentSetLabels::Expression(LabelsExpression {
            labels,
            true_symbols,
            false_symbols,
            unknown_symbols,
        }) => {
            let actual_labels = format!("{}", labels);
            if expected_labels != actual_labels {
                msgs.push(format!(
                    "Expected: {expected_labels}\nActual: {actual_labels}"
                ));
            }
            if expected_truths != true_symbols {
                msgs.push(format!(
                    "Expected: {expected_truths:#?}\nActual: {:#?}",
                    true_symbols
                ));
            }
            if expected_falses != false_symbols {
                msgs.push(format!(
                    "Expected: {expected_falses:#?}\nActual: {:#?}",
                    false_symbols
                ));
            }
            if expected_unknowns != unknown_symbols {
                msgs.push(format!(
                    "Expected: {expected_unknowns:#?}\nActual: {:#?}",
                    unknown_symbols
                ));
            }
        }
    }
    match msgs.is_empty() {
        true => eprintln!("Passed: {labels}\n{given_str}\n"),
        false => eprintln!("Failed: {labels}\n{given_str}\n{}\n", msgs.join("\n")),
    }
}

fn label_var(not: bool, idx: usize) -> LabelItem {
    LabelItem::Item {
        not,
        comp: ComponentSymbol {
            idx,
            args: ComponentMacroArgs::from(&Vec::new()),
        },
    }
}

pub fn test_labels() {
    let args = ComponentMacroArgs::from(&Vec::new());
    // (1 && (0 || !1 || !0))
    let labels = LabelItem::Expression {
        op: LabelOp::And,
        items: vec![
            label_var(false, 1),
            LabelItem::Expression {
                op: LabelOp::Or,
                items: vec![label_var(false, 0), label_var(true, 1), label_var(true, 0)],
            },
        ],
    };
    labels_eq(
        &mut labels.clone(),
        HashMap::new(),
        "(1 && (0 || !0))".to_string(),
        vec![ComponentSymbol { idx: 1, args }],
        vec![],
        vec![ComponentSymbol { idx: 0, args }],
    );
    labels_eq(
        &mut labels.clone(),
        hash_map!({
            ComponentSymbol {
                idx: 0,
                args,
            } => true
        }),
        "(1)".to_string(),
        vec![ComponentSymbol { idx: 1, args }],
        vec![],
        vec![],
    );

    test_labels2();
}

fn test_labels2() {
    let args = ComponentMacroArgs::from(&Vec::new());
    // 0 && ((1) || !0 || !1) && 1
    let labels = LabelItem::Expression {
        op: LabelOp::And,
        items: vec![
            label_var(false, 0),
            LabelItem::Expression {
                op: LabelOp::Or,
                items: vec![label_var(false, 1), label_var(true, 0), label_var(true, 1)],
            },
            label_var(false, 1),
        ],
    };
    labels_eq(
        &mut labels.clone(),
        hash_map!({
            ComponentSymbol {
                idx: 0,
                args,
            } => true
        }),
        "(1)".to_string(),
        vec![ComponentSymbol { idx: 1, args }],
        vec![],
        vec![],
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
