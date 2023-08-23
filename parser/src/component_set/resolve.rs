use diagnostic::{err, CombineResults, ZipResults};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use std::{collections::VecDeque, fmt::Display, ops::Neg};
use syn::{spanned::Spanned, Error, Token};

use super::{
    labels::{ComponentSetLabels, LabelSymbol, LabelsExpression},
    parse::{AstComponentSet, AstComponentSetItem, AstLabelItem, LabelOp},
};
use crate::parse::{
    resolve_path, ComponentSymbol, DiscardSymbol, ItemPath, ItemSpan, MatchSymbol, ModInfo,
    ModInfoMut, WithItemSpan,
};
use shared::{
    syn::{
        error::{CriticalResult, MutateResults, Result, ToError},
        get_fn_name, parse_tokens, ToRange,
    },
    traits::{Call, CollectVec, CollectVecInto, MapOr, PushInto, ThenNone},
};

// Resolve Ast structs to actual items
#[derive(Debug, Clone)]
pub enum LabelItem {
    Item {
        not: bool,
        sym: LabelSymbol,
    },
    Expression {
        op: LabelOp,
        items: Vec<LabelItem>,
        span: ItemSpan,
    },
}

impl LabelItem {
    fn resolve(item: AstLabelItem, (m, cr, crates): ModInfo) -> CriticalResult<Self> {
        let span = ItemSpan::new(cr, m, *item.span());
        match item {
            AstLabelItem::Item { not, ty, .. } => resolve_path(ty, (m, cr, crates))
                .expect_component()
                .discard_symbol()
                .with_item_span(&span)
                .map(|comp| Self::Item {
                    not,
                    sym: LabelSymbol { comp, span },
                }),
            AstLabelItem::Expression { op, items, .. } => items
                .into_iter()
                .map_vec_into(|item| Self::resolve(item, (m, cr, crates)))
                .combine_results()
                .with_item_span(&span)
                .map(|items| Self::Expression { op, items, span }),
        }
    }

    pub fn span(&self) -> &ItemSpan {
        match self {
            LabelItem::Item {
                sym: LabelSymbol { span, .. },
                ..
            }
            | LabelItem::Expression { span, .. } => span,
        }
    }
}

impl std::fmt::Display for LabelItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LabelItem::Item { not, sym, .. } => f.write_fmt(format_args!(
                "{}{}",
                if *not { "!" } else { "" },
                sym.comp.idx
            )),
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
    pub ty: String,
    pub sym: LabelSymbol,
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
    ) -> CriticalResult<Self> {
        let span = ItemSpan::new(cr, m, span);
        resolve_path(ty.path.to_vec(), (m, cr, crates))
            .expect_component()
            .discard_symbol()
            .with_item_span(&span)
            .map(|comp| Self {
                var,
                ty: ty.path.join("::"),
                sym: LabelSymbol { comp, span },
                ref_cnt,
                is_mut,
                is_opt,
            })
    }

    pub fn get_fn(&self) -> syn::Ident {
        get_fn_name(
            if self.sym.comp.args.is_singleton {
                "get_values"
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
    pub span: Span,
}

impl ComponentSet {
    // Gets first non-optional arg, precedence given to singleton args
    pub fn first_arg(&self) -> Option<&ComponentSetItem> {
        self.args
            .iter()
            .find(|item| item.sym.comp.args.is_singleton && !item.is_opt)
            .or(self.args.iter().find(|item| !item.is_opt))
    }

    // Gets first required true label, precedence given to singleton labels
    pub fn first_label(&self) -> Option<&LabelSymbol> {
        match &self.labels {
            Some(ComponentSetLabels::Expression(LabelsExpression { true_symbols, .. })) => {
                true_symbols
                    .iter()
                    .find(|sym| sym.comp.args.is_singleton)
                    .or(true_symbols.first())
            }
            _ => None,
        }
    }

    // Gets first arg or required true label, precedence given to singleton labels
    pub fn first_arg_label(&self) -> Option<&LabelSymbol> {
        match (self.first_arg(), self.first_label()) {
            (Some(arg), Some(sym)) => {
                match (arg.sym.comp.args.is_singleton, sym.comp.args.is_singleton) {
                    (true, _) | (false, false) => Some(&arg.sym),
                    (false, true) => Some(sym),
                }
            }
            (Some(ComponentSetItem { sym, .. }), None) | (None, Some(sym)) => Some(sym),
            (None, None) => None,
        }
    }

    pub fn has_singleton(&self) -> bool {
        self.first_arg_label()
            .is_some_and(|sym| sym.comp.args.is_singleton)
    }

    pub fn parse(tokens: TokenStream, (m, cr, crates): ModInfo) -> Result<Self> {
        let span = tokens.span();
        parse_tokens(tokens).and_then(|cs| Self::resolve(cs, (m, cr, crates)))
    }

    fn resolve(
        AstComponentSet {
            ident,
            args,
            labels,
            label_ident,
        }: AstComponentSet,
        (m, cr, crates): ModInfo,
    ) -> Result<Self> {
        args.into_iter()
            .map_vec_into(|arg| ComponentSetItem::resolve(arg, (m, cr, crates)))
            .combine_results()
            .zip(labels.map_or(Ok(None), |l| {
                LabelItem::resolve(l, (m, cr, crates)).map(|t| Some(t))
            }))
            .map(|(args, labels)| {
                let mut warnings = Vec::new();
                let labels = labels.map(|item| {
                    item.evaluate_labels(
                        args.iter()
                            .filter_map(|comp| comp.is_opt.then_none((comp.sym, true)))
                            .collect(),
                    )
                    .call_into(|labels| {
                        match labels {
                            ComponentSetLabels::Constant(val) => {
                                warnings.push(
                                    ItemSpan::new(
                                        cr,
                                        m,
                                        label_ident.as_ref().unwrap_or(&ident).span(),
                                    )
                                    .warning(format!(
                                        "Label expression is {} true",
                                        val.map_or("always", "never")
                                    )),
                                );
                            }
                            _ => (),
                        }
                        labels
                    })
                });
                err(
                    Self {
                        path: ItemPath::new(cr.idx, m.path.to_vec().push_into(ident.to_string())),
                        labels,
                        args,
                        span: ident.span(),
                    },
                    warnings,
                )
            })
    }
}

impl std::fmt::Display for ComponentSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{{\n{}\n{:#?}\nLabels: {}\n}}",
            self.path.to_string(),
            self.args,
            self.labels
                .as_ref()
                .map_or(String::new(), |cs_labels| format!("{}", cs_labels))
        ))
    }
}
