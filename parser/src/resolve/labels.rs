use std::{collections::HashMap, hash::Hash};

use shared::{
    hash_map,
    util::{Call, Discard, JoinMap, MapNone},
};

use crate::parse::ComponentSymbol;

use super::component_set::{LabelItem, LabelOp};

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

// impl MustBe {
//     fn combine(self, other: MustBe, op: LabelOp, truth: Option<&bool>) -> Self {
//         if let Some(truth) = truth {
//             match (self, other) {
//                 (MustBe::Value(v), _) | (_, MustBe::Value(v)) if v != *truth => {
//                     eprintln!("Value cannot contradict ground truth")
//                 }
//                 _ => (),
//             }
//         }

//         let truth_or_unknown = truth.map_or(MustBe::Unknown, |t| MustBe::Value(*t));

//         match (self, other) {
//             (_, MustBe::Impossible) | (MustBe::Impossible, _) => MustBe::Impossible,
//             (MustBe::Value(v1), MustBe::Value(v2)) => match v1 == v2 {
//                 // Both expressions require the same value
//                 true => MustBe::Value(v1),
//                 // Expressions require opposite values
//                 false => match op {
//                     LabelOp::And => MustBe::Impossible,
//                     // Don't need both expression to be true
//                     LabelOp::Or => truth_or_unknown,
//                 },
//             },
//             // Both expressions are unknown, ground truth value takes precedence
//             (MustBe::Unknown, MustBe::Unknown) => truth_or_unknown,
//             // One expression is unknown
//             (MustBe::Value(v), MustBe::Unknown) | (MustBe::Unknown, MustBe::Value(v)) => match op {
//                 // With 'and', the known expression must still be true
//                 LabelOp::And => MustBe::Value(v),
//                 LabelOp::Or => truth_or_unknown,
//             },
//         }
//     }
// }

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

    pub fn get_symbols(
        &self,
        mut truth: HashMap<ComponentSymbol, bool>,
    ) -> (ExpressionResult, HashMap<ComponentSymbol, bool>) {
        let mut truths = HashMap::new();
        loop {
            match self.get_symbols_impl(&truth) {
                ExpressionResult::Literal(v) => return (ExpressionResult::Literal(v), truths),
                ExpressionResult::Expression(symbs) => {
                    let new_truth = symbs
                        .iter()
                        .filter_map(|(sym, must_be)| match must_be {
                            MustBe::Value(v) => Some((*sym, *v)),
                            MustBe::Unknown => None,
                        })
                        .collect();

                    if new_truth == truth {
                        return (ExpressionResult::Expression(symbs), truths);
                    } else {
                        truth = new_truth;
                        for (sym, v) in &truth {
                            truths.insert(*sym, *v);
                        }
                    }
                }
            }
        }
    }

    fn get_symbols_impl(&self, truth: &HashMap<ComponentSymbol, bool>) -> ExpressionResult {
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
                Self::combine_symbols(items.map_vec(|item| item.get_symbols_impl(truth)), *op)
            }
        }
    }
}
