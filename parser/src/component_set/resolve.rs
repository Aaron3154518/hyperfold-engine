use std::{collections::VecDeque, fmt::Display, ops::Neg};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, Error, Token};

use super::{
    labels::{ComponentSetLabels, LabelsExpression},
    parse::{AstComponentSet, AstComponentSetItem, AstLabelItem, LabelOp},
};
use crate::{
    parse::{resolve_path, ComponentSymbol, DiscardSymbol, ItemPath, MatchSymbol, ModInfo},
    utils::{
        syn::{parse_tokens, ToRange},
        warn, Msg, MsgResult, ParseMsg, ToMsg,
    },
};
use shared::{
    msg_result::{CombineMsgs, MsgTrait, Zip2Msgs},
    traits::{Call, CollectVec, CollectVecInto, PushInto, ThenNone},
};

// Resolve Ast structs to actual items
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LabelItem {
    Item { not: bool, comp: ComponentSymbol },
    Expression { op: LabelOp, items: Vec<LabelItem> },
}

impl LabelItem {
    fn resolve(item: AstLabelItem, (m, cr, crates): ModInfo) -> MsgResult<Self> {
        match item {
            AstLabelItem::Item { not, ty, span } => resolve_path(ty, (m, cr, crates))
                .expect_component_in_mod(m, &span)
                .discard_symbol()
                .map(|comp| Self::Item { not, comp: comp }),
            AstLabelItem::Expression { op, items } => items
                .into_iter()
                .map_vec_into(|item| Self::resolve(item, (m, cr, crates)))
                .combine_msgs()
                .map(|items| Self::Expression { op, items }),
        }
    }
}

impl std::fmt::Display for LabelItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LabelItem::Item { not, comp } => {
                f.write_fmt(format_args!("{}{}", if *not { "!" } else { "" }, comp.idx))
            }
            LabelItem::Expression { op, items } => f.write_fmt(format_args!(
                "({})",
                items
                    .map_vec(|i| format!("{i}"))
                    .join(format!(" {op} ").as_str())
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComponentSetItem {
    pub var: String,
    pub comp: ComponentSymbol,
    pub ref_cnt: usize,
    pub is_mut: bool,
    pub is_opt: bool,
}

impl ComponentSetItem {
    fn resolve(
        AstComponentSetItem {
            var,
            ty,
            ref_cnt,
            is_mut,
            is_opt,
            span,
        }: AstComponentSetItem,
        (m, cr, crates): ModInfo,
    ) -> MsgResult<Self> {
        resolve_path(ty.path.to_vec(), (m, cr, crates))
            .expect_component_in_mod(m, &span)
            .discard_symbol()
            .map(|comp| Self {
                var,
                comp,
                ref_cnt,
                is_mut,
                is_opt,
            })
    }
}

#[derive(Debug)]
pub struct ComponentSet {
    pub path: ItemPath,
    pub args: Vec<ComponentSetItem>,
    pub labels: Option<ComponentSetLabels>,
}

impl ComponentSet {
    // Gets first non-optional arg, precedence given to singleton args
    pub fn first_arg(&self) -> Option<&ComponentSetItem> {
        self.args
            .iter()
            .find(|item| item.comp.args.is_singleton && !item.is_opt)
            .or(self.args.iter().find(|item| !item.is_opt))
    }

    // Gets first required true label, precedence given to singleton labels
    pub fn first_label(&self) -> Option<&ComponentSymbol> {
        match &self.labels {
            Some(ComponentSetLabels::Expression(LabelsExpression { true_symbols, .. })) => {
                true_symbols
                    .iter()
                    .find(|comp| comp.args.is_singleton)
                    .or(true_symbols.first())
            }
            _ => None,
        }
    }

    // Gets first arg or required true label, precedence given to singleton labels
    pub fn first_arg_label(&self) -> Option<&ComponentSymbol> {
        match (self.first_arg(), self.first_label()) {
            (Some(arg), Some(comp)) => match (arg.comp.args.is_singleton, comp.args.is_singleton) {
                (true, _) | (false, false) => Some(&arg.comp),
                (false, true) => Some(comp),
            },
            (Some(ComponentSetItem { comp, .. }), None) | (None, Some(comp)) => Some(comp),
            (None, None) => None,
        }
    }

    pub fn has_singleton(&self) -> bool {
        self.first_arg_label()
            .is_some_and(|comp| comp.args.is_singleton)
    }

    pub fn parse(tokens: TokenStream, (m, cr, crates): ModInfo) -> MsgResult<Self> {
        let span = tokens.span();
        parse_tokens(tokens)
            .for_mod(m)
            .and_then(|cs| Self::resolve(cs, (m, cr, crates)))
    }

    fn resolve(
        AstComponentSet {
            ident,
            args,
            labels,
        }: AstComponentSet,
        (m, cr, crates): ModInfo,
    ) -> MsgResult<Self> {
        args.into_iter()
            .map_vec_into(|arg| ComponentSetItem::resolve(arg, (m, cr, crates)))
            .combine_msgs()
            .zip(labels.map_or(Ok(None), |l| {
                LabelItem::resolve(l, (m, cr, crates)).map(|t| Some(t))
            }))
            .map(|(args, labels)| {
                let labels = labels.map(|labels| {
                    labels
                        .evaluate_labels(
                            args.iter()
                                .filter_map(|sym| sym.is_opt.then_none((sym.comp, true)))
                                .collect(),
                        )
                        .call_into(|labels| {
                            match labels {
                                ComponentSetLabels::Constant(false) => warn(&format!(
                                    "In Component set '{ident}': Label expression is never true"
                                )),
                                _ => (),
                            }
                            labels
                        })
                });
                Self {
                    path: ItemPath {
                        cr_idx: cr.idx,
                        path: m.path.to_vec().push_into(ident),
                    },
                    labels,
                    args,
                }
            })
    }
}

impl std::fmt::Display for ComponentSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{{\n{}\n{:#?}\nLabels: {}\n}}",
            self.path.path.join("::"),
            self.args,
            self.labels
                .as_ref()
                .map_or(String::new(), |cs_labels| format!("{}", cs_labels))
        ))
    }
}
