use std::{collections::HashMap, hash::Hash};

use shared::{
    hash_map,
    util::{Call, Discard, JoinMap, JoinMapInto, MapNone},
};

use crate::parse::ComponentSymbol;

use super::{
    component_set::{LabelItem, LabelOp},
    util::MsgsResult,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MustBe {
    Value(bool),
    Unknown,
}

impl std::fmt::Display for MustBe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(arg0) => write!(f, "{}", if *arg0 { "True" } else { "False" }),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl std::fmt::Debug for MustBe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

pub type SymbolMap = HashMap<ComponentSymbol, MustBe>;
pub enum ExpressionResult {
    Literal(bool),
    Expression(SymbolMap),
}

// TODO: Modify label expressions
impl LabelItem {
    fn combine_symbols(expressions: Vec<ExpressionResult>, op: LabelOp) -> ExpressionResult {
        // Filter out literals
        let mut symbs = Vec::new();
        for res in expressions {
            match res {
                ExpressionResult::Literal(val) => match (op, val) {
                    (LabelOp::And, true) => (),
                    (LabelOp::Or, false) => (),
                    (LabelOp::And, false) => return ExpressionResult::Literal(false),
                    (LabelOp::Or, true) => return ExpressionResult::Literal(true),
                },
                ExpressionResult::Expression(sym_map) => symbs.push(sym_map),
            }
        }

        // Check any expressions left
        let mut sym_it = symbs.into_iter();
        let mut symbs = match sym_it.next() {
            Some(first) => first,
            None => {
                return ExpressionResult::Literal(match op {
                    // All expressions are true
                    LabelOp::And => true,
                    // All expression are false
                    LabelOp::Or => false,
                });
            }
        };

        // Update remaining symbols
        match op {
            LabelOp::And => {
                for next_symbs in sym_it {
                    for (sym, next_must_be) in next_symbs {
                        // match symbs.contains_key(&sym) {
                        //     true => {
                        //         if let Some(must_be) = symbs.get_mut(&sym) {
                        //             *must_be = match (*must_be, next_must_be) {
                        //                 // Contradication
                        //                 (MustBe::Value(v1), MustBe::Value(v2)) if v1 != v2 => {
                        //                     return ExpressionResult::Literal(false)
                        //                 }
                        //                 (MustBe::Value(v), _) | (_, MustBe::Value(v)) => {
                        //                     MustBe::Value(v)
                        //                 }
                        //                 (MustBe::Unknown, MustBe::Unknown) => MustBe::Unknown,
                        //             };
                        //         } else {
                        //             symbs.insert(sym, next_must_be).discard()
                        //         }
                        //     }
                        //     false => symbs.insert(sym, next_must_be).discard(),
                        // }

                        // Update symbols
                        match symbs.insert(sym, next_must_be) {
                            Some(must_be) => {
                                symbs.insert(
                                    sym,
                                    match (must_be, next_must_be) {
                                        // Contradication
                                        (MustBe::Value(v1), MustBe::Value(v2)) if v1 != v2 => {
                                            return ExpressionResult::Literal(false)
                                        }
                                        (MustBe::Value(v), _) | (_, MustBe::Value(v)) => {
                                            MustBe::Value(v)
                                        }
                                        (MustBe::Unknown, MustBe::Unknown) => MustBe::Unknown,
                                    },
                                );
                            }
                            None => (),
                        }
                    }
                }
            }
            LabelOp::Or => {
                for mut next_symbs in sym_it {
                    // Symbols not in the expression are unknown
                    for (sym, must_be) in &symbs {
                        if !next_symbs.contains_key(sym) {
                            next_symbs.insert(*sym, MustBe::Unknown);
                        }
                    }
                    // Update symbols
                    for (sym, next_must_be) in next_symbs {
                        match symbs.insert(sym, next_must_be) {
                            Some(must_be) => {
                                symbs.insert(
                                    sym,
                                    match (must_be, next_must_be) {
                                        (MustBe::Value(v1), MustBe::Value(v2)) if v1 == v2 => {
                                            MustBe::Value(v1)
                                        }
                                        _ => MustBe::Unknown,
                                    },
                                );
                            }
                            None => (),
                        }
                    }
                }
            }
        }

        ExpressionResult::Expression(symbs)
    }

    pub fn evaluate_labels(
        &self,
        mut truth: HashMap<ComponentSymbol, bool>,
    ) -> Option<ComponentSetLabels> {
        let mut true_symbols = Vec::new();
        let mut false_symbols = Vec::new();
        let mut symbols = Vec::new();
        loop {
            match self.get_symbols(&truth) {
                ExpressionResult::Literal(true) => {
                    return {
                        Some(ComponentSetLabels {
                            true_symbols,
                            false_symbols,
                            unknown_symbols: Vec::new(),
                            labels: LabelItem::Expression {
                                op: LabelOp::And,
                                items: symbols,
                            },
                        })
                    }
                }
                ExpressionResult::Literal(false) => return None,
                ExpressionResult::Expression(symbs) => {
                    let new_truth = symbs
                        .iter()
                        .filter_map(|(sym, must_be)| match must_be {
                            MustBe::Value(v) => Some((*sym, *v)),
                            MustBe::Unknown => None,
                        })
                        .collect();

                    if new_truth == truth {
                        return Some(ComponentSetLabels {
                            true_symbols,
                            false_symbols,
                            unknown_symbols: symbs.filter_map_vec_into(|(sym, v)| match v {
                                MustBe::Unknown => Some(sym),
                                _ => None,
                            }),
                            labels: LabelItem::Expression {
                                op: LabelOp::And,
                                items: symbols,
                            },
                        });
                    } else {
                        truth = new_truth;
                        for (sym, v) in &truth {
                            match v {
                                true => true_symbols.push(*sym),
                                false => false_symbols.push(*sym),
                            }
                            symbols.push(LabelItem::Item {
                                not: *v,
                                comp: *sym,
                            });
                        }
                    }
                }
            }
        }
    }

    fn get_symbols(&self, truth: &HashMap<ComponentSymbol, bool>) -> ExpressionResult {
        match self {
            LabelItem::Item { not, comp } => {
                let must_be = !not;
                match truth.get(&comp) {
                    Some(truth) => ExpressionResult::Literal(truth == &must_be),
                    None => {
                        ExpressionResult::Expression(hash_map!({*comp => MustBe::Value(must_be)}))
                    }
                }
            }
            LabelItem::Expression { op, items } => {
                Self::combine_symbols(items.map_vec(|item| item.get_symbols(truth)), *op)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ComponentSetLabels {
    pub labels: LabelItem,
    pub true_symbols: Vec<ComponentSymbol>,
    pub false_symbols: Vec<ComponentSymbol>,
    pub unknown_symbols: Vec<ComponentSymbol>,
}

impl ComponentSetLabels {
    pub fn iter_symbols<'a>(&'a self) -> impl Iterator<Item = &'a ComponentSymbol> {
        self.true_symbols
            .iter()
            .chain(self.false_symbols.iter())
            .chain(self.unknown_symbols.iter())
    }
}
