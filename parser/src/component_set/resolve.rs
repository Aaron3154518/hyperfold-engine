use diagnostic::{CombineResults, ZipResults};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use std::{collections::VecDeque, fmt::Display, ops::Neg};
use syn::{spanned::Spanned, Error, Token};

use super::{
    labels::{ComponentSetLabels, LabelsExpression},
    parse::{AstComponentSet, AstComponentSetItem, AstLabelItem, LabelOp},
};
use crate::parse::{
    resolve_path, ComponentSymbol, DiscardSymbol, ItemPath, MatchSymbol, ModInfo, ModInfoMut,
};
use shared::{
    syn::{
        error::{AddSpan, SpannedResult},
        get_fn_name, parse_tokens, ToRange,
    },
    traits::{Call, CollectVec, CollectVecInto, PushInto, ThenNone},
};

// Resolve Ast structs to actual items
#[derive(Debug, Clone)]
pub enum LabelItem {
    Item {
        not: bool,
        comp: ComponentSymbol,
        span: Span,
    },
    Expression {
        op: LabelOp,
        items: Vec<LabelItem>,
        span: Span,
    },
}

impl LabelItem {
    fn resolve(item: AstLabelItem, (m, cr, crates): ModInfo) -> SpannedResult<Self> {
        match item {
            AstLabelItem::Item { not, ty, span } => resolve_path(ty, (m, cr, crates))
                .expect_component()
                .discard_symbol()
                .add_span(&m.span)
                .map(|comp| Self::Item { not, comp, span }),
            AstLabelItem::Expression { op, items, span } => items
                .into_iter()
                .map_vec_into(|item| Self::resolve(item, (m, cr, crates)))
                .combine_results()
                .map(|items| Self::Expression { op, items, span }),
        }
    }

    pub fn span(&self) -> &Span {
        match self {
            LabelItem::Item { span, .. } | LabelItem::Expression { span, .. } => span,
        }
    }
}

impl std::fmt::Display for LabelItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LabelItem::Item { not, comp, .. } => {
                f.write_fmt(format_args!("{}{}", if *not { "!" } else { "" }, comp.idx))
            }
            LabelItem::Expression { op, items, .. } => f.write_fmt(format_args!(
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
    ) -> SpannedResult<Self> {
        resolve_path(ty.path.to_vec(), (m, cr, crates))
            .expect_component()
            .discard_symbol()
            .add_span(&m.span)
            .map(|comp| Self {
                var,
                comp,
                ref_cnt,
                is_mut,
                is_opt,
            })
    }

    pub fn get_fn(&self) -> syn::Ident {
        get_fn_name(
            if self.comp.args.is_singleton {
                "get_value"
            } else {
                "get"
            },
            self.is_mut,
        )
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

    pub fn parse(tokens: TokenStream, (m, cr, crates): ModInfo) -> SpannedResult<Self> {
        let span = tokens.span();
        parse_tokens(tokens).and_then(|cs| Self::resolve(cs, (m, cr, crates)))
    }

    fn resolve(
        AstComponentSet {
            ident,
            args,
            labels,
        }: AstComponentSet,
        (m, cr, crates): ModInfo,
    ) -> SpannedResult<Self> {
        args.into_iter()
            .map_vec_into(|arg| ComponentSetItem::resolve(arg, (m, cr, crates)))
            .combine_results()
            .zip(labels.map_or(Ok(None), |l| {
                LabelItem::resolve(l, (m, cr, crates)).map(|t| Some(t))
            }))
            .map(|(args, labels)| {
                let labels = labels.map(|item| {
                    item.evaluate_labels(
                        args.iter()
                            .filter_map(|sym| sym.is_opt.then_none((sym.comp, true)))
                            .collect(),
                    )
                    .call_into(|labels| {
                        match labels {
                            // TODO: warning
                            ComponentSetLabels::Constant(false) => {
                                // m.warn(
                                //     &format!(
                                //     "In Component set '{ident}': Label expression is never true"
                                // ),
                                //     item.span(),
                                // );
                            }
                            _ => (),
                        }
                        labels
                    })
                });
                Self {
                    path: ItemPath::new(cr.idx, m.path.to_vec().push_into(ident)),
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
