use std::collections::HashMap;

use super::component_set::{LabelItem, LabelOp};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MustBe {
    True,
    False,
    Unknown,
    Impossible,
}

impl MustBe {
    fn combine(self, other: MustBe, op: &LabelOp) -> Self {
        match (self, other) {
            (_, MustBe::Impossible) | (MustBe::Impossible, _) => MustBe::Impossible,
            // Both expressions require the same value
            (MustBe::True, MustBe::True) => MustBe::True,
            (MustBe::False, MustBe::False) => MustBe::False,
            // The expressions require opposite values, so they can't both be true
            (MustBe::True, MustBe::False) | (MustBe::False, MustBe::True) => match op {
                LabelOp::And => MustBe::Impossible,
                LabelOp::Or => MustBe::Unknown,
            },
            // One expression is unknown, so default to the other expression if and'd
            (s, MustBe::Unknown) | (MustBe::Unknown, s) => match op {
                LabelOp::And => s,
                LabelOp::Or => MustBe::Unknown,
            },
        }
    }
}

impl LabelItem {
    pub fn get_symbols(&self) -> HashMap<usize, MustBe> {
        match self {
            LabelItem::Item { not, c_idx } => {
                [(*c_idx, if *not { MustBe::False } else { MustBe::True })]
                    .into_iter()
                    .collect()
            }
            LabelItem::Expression { op, items } => {
                let mut iter = items.iter();
                // Start with values of first item
                let mut map = match iter.next() {
                    Some(item) => item.get_symbols(),
                    None => HashMap::new(),
                };
                for item in iter {
                    let symbs = item.get_symbols();
                    // Add/update symbols of next item
                    for (i, must_be) in &symbs {
                        if !map.contains_key(i) {
                            map.insert(*i, must_be.combine(MustBe::Unknown, op));
                        } else {
                            match map.get_mut(i) {
                                Some(curr_must_be) => {
                                    *curr_must_be = must_be.combine(*curr_must_be, op)
                                }
                                None => panic!("Failed to get must be from map"),
                            }
                        }
                    }
                    if *op == LabelOp::Or {
                        // All other symbols are now unknown (previous expressions could be false)
                        for (i, must_be) in map.iter_mut() {
                            if !symbs.contains_key(i) {
                                *must_be = match must_be {
                                    MustBe::Impossible => MustBe::Impossible,
                                    _ => MustBe::Unknown,
                                }
                            }
                        }
                    }
                }
                map
            }
        }
    }
}
