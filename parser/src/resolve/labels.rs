use std::{collections::HashMap, hash::Hash};

use shared::util::{JoinMap, MapNone};

use crate::parse::ComponentSymbol;

use super::component_set::{LabelItem, LabelOp};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MustBe {
    Value(bool),
    Unknown,
    Impossible,
}

impl std::fmt::Display for MustBe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(arg0) => write!(f, "{}", if *arg0 { "True" } else { "False" }),
            Self::Unknown => write!(f, "Unknown"),
            Self::Impossible => write!(f, "Impossible"),
        }
    }
}

impl std::fmt::Debug for MustBe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl MustBe {
    fn combine(self, other: MustBe, op: LabelOp, truth: Option<&bool>) -> Self {
        if let Some(truth) = truth {
            match (self, other) {
                (MustBe::Value(v), _) | (_, MustBe::Value(v)) if v != *truth => {
                    eprintln!("Value cannot contradict ground truth")
                }
                _ => (),
            }
        }

        let truth_or_unknown = truth.map_or(MustBe::Unknown, |t| MustBe::Value(*t));

        match (self, other) {
            (_, MustBe::Impossible) | (MustBe::Impossible, _) => MustBe::Impossible,
            (MustBe::Value(v1), MustBe::Value(v2)) => match v1 == v2 {
                // Both expressions require the same value
                true => MustBe::Value(v1),
                // Expressions require opposite values
                false => match op {
                    LabelOp::And => MustBe::Impossible,
                    // Don't need both expression to be true
                    LabelOp::Or => truth_or_unknown,
                },
            },
            // Both expressions are unknown, ground truth value takes precedence
            (MustBe::Unknown, MustBe::Unknown) => truth_or_unknown,
            // One expression is unknown
            (MustBe::Value(v), MustBe::Unknown) | (MustBe::Unknown, MustBe::Value(v)) => match op {
                // With 'and', the known expression must still be true
                LabelOp::And => MustBe::Value(v),
                LabelOp::Or => truth_or_unknown,
            },
        }
    }
}

pub type SymbolMap = HashMap<ComponentSymbol, MustBe>;

impl LabelItem {
    fn combine_symbols(
        mut symbs: Vec<SymbolMap>,
        op: LabelOp,
        truth: &HashMap<ComponentSymbol, bool>,
    ) -> SymbolMap {
        // Remove impossible items
        if op == LabelOp::Or {
            // If all the items are impossible, process all the symbols to propogate the impossibles
            // Otherwise, only process the possible items
            let or_symbs = symbs
                .iter()
                .enumerate()
                .filter_map(|(i, symb)| {
                    symb.iter()
                        .position(|(_, must_be)| *must_be == MustBe::Impossible)
                        .map_none(|| i)
                })
                .collect::<Vec<_>>();
            if !or_symbs.is_empty() {
                symbs = symbs
                    .into_iter()
                    .enumerate()
                    .filter_map(|(i, sym)| or_symbs.contains(&i).then_some(sym))
                    .collect();
            }
        }

        let mut iter = symbs.into_iter();
        // Start with values of first item
        let mut map = iter.next().unwrap_or(HashMap::new());
        for mut symbs in iter {
            if op == LabelOp::Or {
                // All other symbols are unknown (previous expressions could be false)
                for (c_sym, must_be) in map.iter() {
                    if !symbs.contains_key(c_sym) {
                        symbs.insert(*c_sym, MustBe::Unknown);
                    }
                }
            }
            // Add/update symbols of next item
            for (c_sym, must_be) in &symbs {
                if !map.contains_key(c_sym) {
                    map.insert(
                        *c_sym,
                        must_be.combine(MustBe::Unknown, op, truth.get(c_sym)),
                    );
                } else {
                    match map.get_mut(c_sym) {
                        Some(curr_must_be) => {
                            *curr_must_be = must_be.combine(*curr_must_be, op, truth.get(c_sym))
                        }
                        None => panic!("Failed to get must be from map"),
                    }
                }
            }
        }
        map
    }

    pub fn get_symbols(&self, mut truth: HashMap<ComponentSymbol, bool>) -> SymbolMap {
        loop {
            let mut map = self.get_symbols_impl(&truth);
            let mut new_truth = HashMap::new();
            for (i, symb) in &map {
                match symb {
                    MustBe::Value(v) => {
                        new_truth.insert(*i, *v);
                    }
                    MustBe::Unknown => (),
                    MustBe::Impossible => return map,
                }
            }
            if new_truth == truth {
                return map;
            } else {
                truth = new_truth;
            }
        }
    }

    fn get_symbols_impl(&self, truth: &HashMap<ComponentSymbol, bool>) -> SymbolMap {
        match self {
            LabelItem::Item { not, comp } => {
                let must_be = !not;
                let mut symbs = HashMap::new();
                symbs.insert(
                    *comp,
                    match truth.get(&comp).unwrap_or(&must_be) == &must_be {
                        true => MustBe::Value(must_be),
                        false => MustBe::Impossible,
                    },
                );
                symbs
            }
            LabelItem::Expression { op, items } => Self::combine_symbols(
                items.map_vec(|item| item.get_symbols_impl(truth)),
                *op,
                truth,
            ),
        }
    }

    fn map_symbols_to_truth(symbs: &SymbolMap) -> HashMap<ComponentSymbol, bool> {
        symbs
            .into_iter()
            .filter_map(|(i, v)| match v {
                MustBe::Value(v) => Some((*i, *v)),
                MustBe::Unknown => None,
                MustBe::Impossible => None,
            })
            .collect()
    }
}
