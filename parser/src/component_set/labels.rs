use std::{collections::HashMap, hash::Hash};

use proc_macro2::Span;
use shared::{
    hash_map,
    traits::{Call, CollectVec, CollectVecInto, Discard, MapNone, PushInto},
};

use super::{parse::LabelOp, resolve::LabelItem};
use crate::parse::{ComponentSymbol, ItemSpan};

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

pub type SymbolMap = HashMap<LabelSymbol, MustBe>;
pub type TruthMap = HashMap<LabelSymbol, bool>;

pub enum ExpressionResult {
    Literal(bool),
    Expression {
        labels: LabelItem,
        symbols: SymbolMap,
    },
}

impl LabelItem {
    fn combine_symbols(
        op: &LabelOp,
        items: &Vec<LabelItem>,
        span: ItemSpan,
        truth: &TruthMap,
    ) -> ExpressionResult {
        // Filter out literals
        let mut new_symbols = Vec::new();
        let mut new_labels = Vec::new();
        for item in items {
            match item.get_symbols(truth) {
                ExpressionResult::Literal(val) => {
                    match (op, val) {
                        (LabelOp::And, true) | (LabelOp::Or, false) => (),
                        // Entire expression evaluates to a literal
                        (LabelOp::And, false) => return ExpressionResult::Literal(false),
                        (LabelOp::Or, true) => return ExpressionResult::Literal(true),
                    }
                }
                ExpressionResult::Expression { labels, symbols } => {
                    new_symbols.push(symbols);
                    new_labels.push(labels);
                }
            }
        }

        // Check if there are any expressions left
        let mut sym_it = new_symbols.into_iter();
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
                        match symbs.get_mut(&sym) {
                            Some(must_be) => {
                                *must_be = match (*must_be, next_must_be) {
                                    // Contradication
                                    (MustBe::Value(v1), MustBe::Value(v2)) if v1 != v2 => {
                                        return ExpressionResult::Literal(false)
                                    }
                                    (MustBe::Value(v), _) | (_, MustBe::Value(v)) => {
                                        MustBe::Value(v)
                                    }
                                    (MustBe::Unknown, MustBe::Unknown) => MustBe::Unknown,
                                }
                            }
                            None => symbs.insert(sym, next_must_be).discard(),
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
                    for (sym, next_must_be) in next_symbs {
                        match symbs.get_mut(&sym) {
                            Some(must_be) => {
                                *must_be = match (*must_be, next_must_be) {
                                    (MustBe::Value(v1), MustBe::Value(v2)) if v1 == v2 => {
                                        MustBe::Value(v1)
                                    }
                                    _ => MustBe::Unknown,
                                }
                            }
                            None => symbs.insert(sym, MustBe::Unknown).discard(),
                        }
                    }
                }
            }
        }

        ExpressionResult::Expression {
            labels: match new_labels.split_first() {
                Some((first, [])) => first.clone(),
                _ => LabelItem::Expression {
                    op: *op,
                    items: new_labels,
                    span,
                },
            },
            symbols: symbs,
        }
    }

    fn get_symbols(&self, truth: &TruthMap) -> ExpressionResult {
        match self {
            LabelItem::Item { not, sym } => {
                let must_be = !*not;
                match truth.get(sym) {
                    Some(truth) => ExpressionResult::Literal(truth == &must_be),
                    None => ExpressionResult::Expression {
                        labels: LabelItem::Item {
                            not: *not,
                            sym: *sym,
                        },
                        symbols: hash_map!({*sym => MustBe::Value(must_be)}),
                    },
                }
            }
            LabelItem::Expression { op, items, span } => {
                Self::combine_symbols(op, items, *span, truth)
            }
        }
    }

    pub fn evaluate_labels(&self, mut truth: TruthMap) -> ComponentSetLabels {
        let span = *self.span();
        // Collects symbols which must be true/false
        let mut true_symbols = Vec::new();
        let mut false_symbols = Vec::new();
        // Collects labels items for true/false symbols
        let mut fixed_labels = Vec::new();
        let mut eval_result = self.get_symbols(&truth);
        loop {
            match eval_result {
                ExpressionResult::Literal(true) => {
                    return if fixed_labels.is_empty() {
                        ComponentSetLabels::Constant(true)
                    } else {
                        ComponentSetLabels::Expression(LabelsExpression {
                            labels: LabelItem::Expression {
                                op: LabelOp::And,
                                items: fixed_labels,
                                span,
                            },
                            true_symbols,
                            false_symbols,
                            unknown_symbols: Vec::new(),
                        })
                    }
                }
                ExpressionResult::Literal(false) => return ComponentSetLabels::Constant(false),
                ExpressionResult::Expression { labels, symbols } => {
                    let new_truth = symbols
                        .iter()
                        .filter_map(|(sym, must_be)| match must_be {
                            MustBe::Value(v) => Some((*sym, *v)),
                            MustBe::Unknown => None,
                        })
                        .collect();

                    if new_truth == truth {
                        return ComponentSetLabels::Expression(LabelsExpression {
                            labels: LabelItem::Expression {
                                op: LabelOp::And,
                                items: fixed_labels.push_into(labels),
                                span,
                            },
                            true_symbols,
                            false_symbols,
                            unknown_symbols: symbols.filter_map_vec_into(|(sym, v)| match v {
                                MustBe::Unknown => Some(sym),
                                _ => None,
                            }),
                        });
                    } else {
                        truth = new_truth;
                        for (sym, v) in &truth {
                            match v {
                                true => true_symbols.push(*sym),
                                false => false_symbols.push(*sym),
                            }
                            fixed_labels.push(LabelItem::Item {
                                not: !*v,
                                sym: *sym,
                            });
                        }
                        eval_result = labels.get_symbols(&truth)
                    }
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct LabelSymbol {
    // This contains the span of the original component declaration
    pub comp: ComponentSymbol,
    // This is the span of the component in the label expression
    pub span: ItemSpan,
}

// Used to index with labels
impl Hash for LabelSymbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.comp.idx.hash(state);
    }
}

impl PartialEq for LabelSymbol {
    fn eq(&self, other: &Self) -> bool {
        self.comp.idx == other.comp.idx
    }
}

impl Eq for LabelSymbol {}

#[derive(Debug)]
pub struct LabelsExpression {
    pub labels: LabelItem,
    pub true_symbols: Vec<LabelSymbol>,
    pub false_symbols: Vec<LabelSymbol>,
    pub unknown_symbols: Vec<LabelSymbol>,
}

impl LabelsExpression {
    pub fn iter_symbols<'a>(&'a self) -> impl Iterator<Item = &'a LabelSymbol> {
        self.true_symbols
            .iter()
            .chain(self.false_symbols.iter())
            .chain(self.unknown_symbols.iter())
    }
}

#[derive(Debug)]
pub enum ComponentSetLabels {
    Constant(bool),
    Expression(LabelsExpression),
}

impl ComponentSetLabels {
    pub fn first_true_symbol<'a>(&'a self) -> Option<&'a LabelSymbol> {
        match self {
            ComponentSetLabels::Constant(_) => None,
            ComponentSetLabels::Expression(e) => e.true_symbols.first(),
        }
    }
}

impl std::fmt::Display for ComponentSetLabels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentSetLabels::Constant(v) => write!(f, "{v}"),
            ComponentSetLabels::Expression(e) => write!(f, "{}", e.labels),
        }
    }
}
