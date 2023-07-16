use parse_cfg::Cfg;
use proc_macro2::Span;
use quote::ToTokens;
use syn::spanned::Spanned;

use crate::utils::{syn::use_path_from_vec, CatchErr, MsgResult};
use shared::{
    msg_result::{MsgTrait, ToMsgs},
    traits::NoneOr,
};

#[derive(Clone, Debug)]
pub struct AstAttribute {
    pub path: Vec<String>,
    pub args: Vec<String>,
    pub span: Span,
}

// Parse attributes from engine components
#[derive(Copy, Clone, Debug)]
pub enum EcsAttribute {
    Component,
    Global,
    System,
    Event,
}

impl EcsAttribute {
    fn from(value: String) -> Option<Self> {
        match value.as_str() {
            "component" => Some(EcsAttribute::Component),
            "global" => Some(EcsAttribute::Global),
            "system" => Some(EcsAttribute::System),
            "event" => Some(EcsAttribute::Event),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Attribute {
    Ecs(AstAttribute),
    Cfg(Cfg),
}

impl Attribute {
    fn from(attr: Vec<String>, span: Span) -> Self {
        match attr.join("::").as_str() {
            "cfg" => Self::Cfg(Cfg::Is(String::new())),
            s => Self::Ecs(AstAttribute {
                path: attr,
                args: Vec::new(),
                span,
            }),
        }
    }
}

pub fn get_attributes_if_active(
    attrs: &Vec<syn::Attribute>,
    path: &Vec<String>,
    features: &Vec<String>,
) -> MsgResult<Option<Vec<AstAttribute>>> {
    let mut is_active = true;
    let new_attrs =
        get_attributes(attrs, path)?
            .into_iter()
            .fold(Vec::new(), |mut new_attrs, a| {
                match a {
                    Attribute::Ecs(attr) => new_attrs.push(attr),
                    Attribute::Cfg(cfg) => {
                        is_active = eval_cfg_args(&cfg, features).is_none_or_into(|b| b)
                    }
                }
                new_attrs
            });
    Ok(is_active.then_some(new_attrs))
}

// Returns list of parsed attributes from ast attributes
pub fn get_attributes(
    attrs: &Vec<syn::Attribute>,
    path: &Vec<String>,
) -> MsgResult<Vec<Attribute>> {
    let mut new_attrs = Vec::new();
    let mut errs = Vec::new();
    for a in attrs {
        parse_attr_args(
            Attribute::from(
                use_path_from_vec(
                    path,
                    &a.path()
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect(),
                ),
                a.span(),
            ),
            a,
        )
        .record_err_or(&mut errs, |attr| new_attrs.push(attr));
    }
    errs.err_or(new_attrs)
}

// Check cfg args to make sure we are valid
pub fn eval_cfg_args(cfg: &Cfg, features: &Vec<String>) -> Option<bool> {
    match cfg {
        Cfg::Any(cfgs) => Some(
            cfgs.iter()
                .map(|cfg| eval_cfg_args(&cfg, &features).is_some_and(|b| b))
                .collect::<Vec<_>>()
                .contains(&true),
        ),
        Cfg::All(cfgs) => Some(
            !cfgs
                .iter()
                .map(|cfg| eval_cfg_args(&cfg, &features).is_none_or_into(|b| b))
                .collect::<Vec<_>>()
                .contains(&false),
        ),
        Cfg::Not(cfg) => eval_cfg_args(cfg, &features).map(|b| !b),
        Cfg::Equal(k, v) => {
            if k == "feature" {
                Some(features.contains(&v))
            } else {
                None
            }
        }
        Cfg::Is(_) => None,
    }
}

// Parses arguments to a single ast attribute
fn parse_attr_args(mut attr_type: Attribute, attr: &syn::Attribute) -> MsgResult<Attribute> {
    match &mut attr_type {
        Attribute::Ecs(ast_attr) => match &attr.meta {
            syn::Meta::List(l) => {
                for t in l.to_token_stream() {
                    match t {
                        proc_macro2::TokenTree::Group(g) => {
                            ast_attr.args = g
                                .stream()
                                .into_iter()
                                .filter_map(|tt| match tt {
                                    proc_macro2::TokenTree::Ident(i) => Some(i.to_string()),
                                    _ => None,
                                })
                                .collect();
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        },
        Attribute::Cfg(cfg) => match &attr.meta {
            syn::Meta::List(l) => {
                *cfg = l
                    .to_token_stream()
                    .to_string()
                    .parse()
                    .catch_err("Could not parse cfg_str")?;
            }
            _ => (),
        },
    };
    Ok(attr_type)
}
