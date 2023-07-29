use std::collections::HashMap;

use shared::{hash_map, parsing::ComponentMacroArgs};

use crate::parse::ComponentSymbol;

use super::{labels::LabelsExpression, parse::LabelOp, resolve::LabelItem, ComponentSetLabels};

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
            args: ComponentMacroArgs::default(),
        },
    }
}

pub fn test_labels() {
    let args = ComponentMacroArgs::default();
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
    let args = ComponentMacroArgs::default();
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
