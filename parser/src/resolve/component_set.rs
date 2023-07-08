use std::{collections::VecDeque, fmt::Display, ops::Neg};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use shared::util::{Call, Catch, FindFrom, Get, JoinMap, JoinMapInto, MapNone, NoneOr, PushInto};
use syn::{spanned::Spanned, Error, Token};

use crate::{
    codegen2::CodegenIdents,
    parse::{
        AstCrate, ComponentSymbol, DiscardSymbol, MatchSymbol, ModInfo, SymbolType,
        {AstMod, Symbol},
    },
    resolve::path::resolve_path,
    resolve::util::{CombineMsgs, MsgsResult},
};

use super::{
    constants::{component_set_fn, component_set_var, component_var},
    items_crate::ItemComponent,
    labels::{ComponentSetLabels, LabelsExpression, SymbolMap},
    parse_macro_call::ParseMacroCall,
    path::{ItemPath, ResolveResultTrait},
    util::Zip2Msgs,
    Items, MustBe,
};

macro_rules! err {
    ($token: ident, $msg: literal) => {
        Err(Error::new($token.span(), $msg))
    };

    ($token: ident, $msg: literal, $($es: ident),+) => {
        Err({
            let mut err_ = Error::new($token.span(), $msg);
            $(err_.combine($es);)*
            err_
        })
    };
}

macro_rules! parse_expect {
    ($tokens: ident, $msg: literal) => {
        match $tokens.parse() {
            Ok(t) => t,
            Err(e) => return err!($tokens, $msg, e),
        }
    };

    ($tokens: ident, $type: ty, $msg: literal) => {
        match $tokens.parse::<$type>() {
            Ok(t) => t,
            Err(e) => return err!($tokens, $msg, e),
        }
    };
}

/*
* Pass 1: Parse into expressions
* Grammar:
* Expr -> Item (Op Item)*
* Item -> !*Ident | !*(Expr)
* Op -> && | ||

* Pass 2: Apply DeMorgan's law
          Split expression sequences with left->right precedence for &&
          Flatten
*/

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum LabelOp {
    And,
    Or,
}

impl LabelOp {
    pub fn negate(&mut self) {
        *self = match self {
            LabelOp::And => LabelOp::Or,
            LabelOp::Or => LabelOp::And,
        }
    }

    pub fn quote(&self) -> TokenStream {
        match self {
            LabelOp::And => quote!(&&),
            LabelOp::Or => quote!(||),
        }
    }
}

impl syn::parse::Parse for LabelOp {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input
            .parse::<Token!(&&)>()
            .map(|_| Self::And)
            .or_else(|_| input.parse::<Token!(||)>().map(|_| Self::Or))
    }
}

impl std::fmt::Display for LabelOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            LabelOp::And => "&&",
            LabelOp::Or => "||",
        })
    }
}

#[derive(Clone, Debug)]
enum Expression {
    Item(AstLabelItem),
    Expr {
        first: AstLabelItem,
        noops: Vec<AstLabelItem>,
        ops: Vec<LabelOp>,
    },
}

impl Expression {
    pub fn to_item(self, neg: bool) -> AstLabelItem {
        let mut item = match self {
            Expression::Item(i) => i,
            Expression::Expr { first, noops, ops } => {
                let mut get_item = |items: Vec<AstLabelItem>, op: LabelOp| {
                    if items.len() == 1 {
                        items.into_iter().next().expect("This will not fail")
                    } else {
                        // Combine expressions with the same operation
                        let items = items.into_iter().fold(Vec::new(), |mut items, item| {
                            match item {
                                AstLabelItem::Item { not, ty } => {
                                    items.push(AstLabelItem::Item { not, ty })
                                }
                                AstLabelItem::Expression {
                                    op: exp_op,
                                    items: exp_items,
                                } => {
                                    if op == exp_op {
                                        items.extend(exp_items);
                                    } else {
                                        items.push(AstLabelItem::Expression {
                                            op: exp_op,
                                            items: exp_items,
                                        });
                                    }
                                }
                            }
                            items
                        });
                        AstLabelItem::Expression { op, items }
                    }
                };
                let mut ors = Vec::new();
                let mut ands = vec![first];
                for (item, op) in noops.into_iter().zip(ops.into_iter()) {
                    match op {
                        LabelOp::And => ands.push(item),
                        LabelOp::Or => {
                            ors.push(get_item(ands, LabelOp::And));
                            ands = vec![item];
                        }
                    }
                }
                ors.push(get_item(ands, LabelOp::And));
                get_item(ors, LabelOp::Or)
            }
        };
        if neg {
            item.negate();
        }
        item
    }
}

impl syn::parse::Parse for Expression {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let first = match input.parse() {
            Ok(t) => t,
            Err(e) => return err!(input, "Expected label expression in parentheses", e),
        };

        match input.parse() {
            Ok(op) => {
                let mut noops = vec![parse_expect!(input, "Expected NoOp after Op")];
                let mut ops = vec![op];

                while let Ok(op) = input.parse() {
                    ops.push(op);
                    noops.push(parse_expect!(input, "Expected NoOp after Op"));
                }

                Ok(Self::Expr { first, noops, ops })
            }
            Err(_) => Ok(Self::Item(first)),
        }
    }
}

// Flattened label set
#[derive(Clone, Debug)]
enum AstLabelItem {
    Item {
        not: bool,
        ty: Vec<String>,
    },
    Expression {
        op: LabelOp,
        items: Vec<AstLabelItem>,
    },
}

impl AstLabelItem {
    pub fn negate(&mut self) {
        match self {
            AstLabelItem::Item { not, ty } => *not = !*not,
            AstLabelItem::Expression { op, items } => {
                op.negate();
                items.iter_mut().for_each(|i| i.negate());
            }
        }
    }
}

impl syn::parse::Parse for AstLabelItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut not = false;
        while input.parse::<Token!(!)>().is_ok() {
            not = !not;
        }

        input.parse::<proc_macro2::Group>().map_or_else(
            |_| {
                input.parse::<syn::Type>().map_or_else(
                    |e| err!(input, "Expected parentheses or ident in label", e),
                    |ty| match ty {
                        syn::Type::Path(p) => Ok(Self::Item {
                            not,
                            ty: p
                                .path
                                .segments
                                .iter()
                                .map(|s| s.ident.to_string())
                                .collect(),
                        }),
                        _ => err!(ty, "Invalid type in label"),
                    },
                )
            },
            |g| syn::parse2::<Expression>(g.stream()).map(|e| e.to_item(not)),
        )
    }
}

impl std::fmt::Display for AstLabelItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AstLabelItem::Item { not, ty } => f.write_fmt(format_args!(
                "{}{}",
                if *not { "!" } else { "" },
                ty.join("::")
            )),
            AstLabelItem::Expression { op, items } => f.write_fmt(format_args!(
                "({})",
                items
                    .map_vec(|i| format!("{i}"))
                    .join(format!(" {op} ").as_str())
            )),
        }
    }
}

#[derive(Debug)]
struct AstComponentSetItem {
    var: String,
    ty: ItemPath,
    ref_cnt: usize,
    is_mut: bool,
}

impl AstComponentSetItem {
    pub fn from(var: String, ty: syn::Type) -> syn::Result<Self> {
        let span = ty.span();
        return match ty {
            syn::Type::Path(ty) => Ok(Self {
                var,
                ty: ItemPath {
                    cr_idx: 0,
                    path: ty
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect(),
                },
                ref_cnt: 0,
                is_mut: false,
            }),
            syn::Type::Reference(ty) => match Self::from(var, *ty.elem) {
                Ok(mut i) => {
                    i.ref_cnt += 1;
                    i.is_mut |= ty.mutability.is_some();
                    Ok(i)
                }
                Err(e) => err!(span, "Couldn't parse component arg type", e),
            },
            _ => err!(ty, "Invalid variable type"),
        };
    }
}

impl syn::parse::Parse for AstComponentSetItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // var : type
        let var = match input.parse::<syn::Ident>() {
            Ok(i) => i.to_string(),
            Err(e) => return err!(input, "Expected variable name", e),
        };
        if let Err(e) = input.parse::<syn::token::Colon>() {
            return err!(input, "Expected ':'", e);
        }
        AstComponentSetItem::from(
            var,
            parse_expect!(input, syn::Type, "Expected variable type"),
        )
    }
}

#[derive(Debug)]
struct AstComponentSet {
    ident: String,
    args: Vec<AstComponentSetItem>,
    labels: Option<AstLabelItem>,
}

impl syn::parse::Parse for AstComponentSet {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut _comma: syn::token::Comma;

        let mut labels = None;

        // First ident
        let mut first_ident: syn::Ident = parse_expect!(input, "Expected ident");
        if first_ident == "labels" {
            // Labels
            labels = match input.parse::<proc_macro2::Group>() {
                Ok(g) => {
                    let stream = g.stream();
                    if stream.is_empty() {
                        None
                    } else {
                        Some(match syn::parse2::<Expression>(stream) {
                            Ok(e) => e.to_item(false),
                            Err(e) => return err!(g, "Failed to parse labels", e),
                        })
                    }
                }
                Err(e) => return err!(input, "Expected parentheses after labels", e),
            };
            _comma = parse_expect!(input, "Expected comma");
            first_ident = parse_expect!(input, "Expected ident");
        }

        let mut args = Vec::new();
        while let Result::Ok(_) = input.parse::<syn::token::Comma>() {
            args.push(parse_expect!(input, "Expected argument variable-type pair"));
        }

        Ok(Self {
            ident: first_ident.to_string(),
            args,
            labels,
        })
    }
}

impl std::fmt::Display for AstComponentSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{{\n{:#?}\n{:#?}\nLabels: {}\n}}",
            self.ident,
            self.args,
            self.labels
                .as_ref()
                .map_or(String::new(), |l| format!("{l}"))
        ))
    }
}

// Resolve Ast structs to actual items
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LabelItem {
    Item { not: bool, comp: ComponentSymbol },
    Expression { op: LabelOp, items: Vec<LabelItem> },
}

impl LabelItem {
    fn resolve(item: AstLabelItem, (m, cr, crates): ModInfo) -> MsgsResult<Self> {
        match item {
            AstLabelItem::Item { not, ty } => resolve_path(ty, (m, cr, crates))
                .expect_symbol()
                .expect_component()
                .discard_symbol()
                .map(|c_sym| Self::Item { not, comp: c_sym }),
            AstLabelItem::Expression { op, items } => items
                .into_iter()
                .map_vec_into(|item| Self::resolve(item, (m, cr, crates)))
                .combine_msgs()
                .map(|items| Self::Expression { op, items }),
        }
    }

    fn quote<F>(&self, f: &F) -> TokenStream
    where
        F: Fn(ComponentSymbol) -> syn::Ident,
    {
        match self {
            LabelItem::Item { not, comp } => {
                let not = if *not { quote!(!) } else { quote!() };
                let var = f(*comp);
                quote!(#not #var)
            }
            LabelItem::Expression { op, items } => {
                let vars = items
                    .iter()
                    .map(|i| i.quote(f))
                    .intersperse(op.quote())
                    .collect::<Vec<_>>();
                quote!((#(#vars)*))
            }
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
}

impl ComponentSetItem {
    fn resolve(
        AstComponentSetItem {
            var,
            ty,
            ref_cnt,
            is_mut,
        }: AstComponentSetItem,
        (m, cr, crates): ModInfo,
    ) -> MsgsResult<Self> {
        resolve_path(ty.path.to_vec(), (m, cr, crates))
            .expect_symbol()
            .expect_component()
            .discard_symbol()
            .map(|comp| Self {
                var,
                comp,
                ref_cnt,
                is_mut,
            })
    }
}

#[derive(Debug)]
pub struct ComponentSet {
    pub path: ItemPath,
    pub args: Vec<ComponentSetItem>,
    pub labels: Option<ComponentSetLabels>,
    // First singleton found in args or required in labels
    pub first_singleton: Option<ComponentSymbol>,
}

impl ComponentSet {
    pub fn parse(tokens: TokenStream, (m, cr, crates): ModInfo) -> MsgsResult<Self> {
        syn::parse2(tokens)
            .map_err(|_| {
                vec![format!(
                    "Failed to parse component set in mod: {}",
                    m.path.join("::")
                )]
            })
            .and_then(|cs| Self::resolve(cs, (m, cr, crates)))
    }

    fn resolve(
        AstComponentSet {
            ident,
            args,
            labels,
        }: AstComponentSet,
        (m, cr, crates): ModInfo,
    ) -> MsgsResult<Self> {
        args.into_iter()
            .map_vec_into(|arg| ComponentSetItem::resolve(arg, (m, cr, crates)))
            .combine_msgs()
            .zip(labels.map_or(Ok(None), |l| {
                LabelItem::resolve(l, (m, cr, crates)).map(|t| Some(t))
            }))
            .map(|(args, labels)| {
                let labels = labels.map(|labels| {
                    labels
                        .evaluate_labels(args.iter().map(|sym| (sym.comp, true)).collect())
                        .call_into(|labels| {
                            match labels {
                                ComponentSetLabels::Constant(false) => eprintln!(
                                    "In Component set '{ident}': Label expression is never true"
                                ),
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
                    first_singleton: args
                        .iter()
                        .find(|item| item.comp.args.is_singleton)
                        .map(|item| item.comp)
                        .or_else(|| match &labels {
                            Some(ComponentSetLabels::Expression(e)) => e
                                .true_symbols
                                .iter()
                                .find(|c_sym| c_sym.args.is_singleton)
                                .copied(),
                            _ => None,
                        }),
                    labels,
                    args,
                }
            })
    }

    // TODO: eid need ref
    pub fn quote(
        &self,
        cs_idx: usize,
        ty: &syn::Path,
        filter: &syn::Path,
        intersect: &syn::Path,
        intersect_mut: &syn::Path,
    ) -> TokenStream {
        let eid = CodegenIdents::GenEid.to_ident();
        let eids = CodegenIdents::GenEids.to_ident();
        let comps_var = CodegenIdents::GenCFoo.to_ident();
        let comps_type = CodegenIdents::CFoo.to_ident();

        let fn_name_vec = component_set_fn(cs_idx, true);

        // Remove duplicate arguments
        let mut args = self.args.to_vec();
        args.sort_by_key(|item| item.comp.idx);
        args.dedup_by_key(|item| item.comp.idx);

        let gen_filter_fn = |v, e: &LabelsExpression| {
            let label_expr = e.labels.quote(&|c_sym| component_var(c_sym.idx));
            let label_vars = e.iter_symbols().filter_map_vec_into(|c_sym| {
                args.iter()
                    .find(|c| c.comp.idx == c_sym.idx)
                    .map_none(component_var(c_sym.idx))
            });
            quote!(
                #filter(#v, [#(&#comps_var.#label_vars),*], |[#(#label_vars),*]| #label_expr)
            )
        };

        match self.first_singleton {
            Some(s) => {
                let s = component_var(s.idx);

                // Match statement for components
                let (gets, comps) = args.unzip_vec(|item| {
                    let var = component_var(item.comp.idx);
                    (
                        match item.is_mut {
                            true => quote!(#comps_var.#var.get_mut),
                            false => quote!(#comps_var.#var.get),
                        },
                        var,
                    )
                });

                // Args to component set struct
                let (arg_vars, arg_vals) = self.args.unzip_vec(|item| {
                    (format_ident!("{}", item.var), component_var(item.comp.idx))
                });

                let create_cs = quote!(
                    if let (#(Some(#comps),)*) = (#(#gets(&k),)*) {
                        return Some(#ty {
                            #eid: k,
                            #(#arg_vars: #arg_vals),*
                        });
                    }
                );

                // If statement for label expression
                let body = match &self.labels {
                    None | Some(ComponentSetLabels::Constant(true)) => Some(quote!(
                        #create_cs
                    )),
                    Some(ComponentSetLabels::Constant(false)) => None,
                    Some(ComponentSetLabels::Expression(e)) => {
                        let filter = gen_filter_fn(quote!(vec![(&k, ())]), e);
                        Some(quote!(
                            if !#filter.is_empty() {
                                #create_cs
                            }
                        ))
                    }
                }
                .map_or(quote!(None), |body| {
                    quote!(
                        if let Some(k) = #s.get_key() {
                            #body
                        }
                        None
                    )
                });

                let fn_name = component_set_fn(cs_idx, false);

                quote!(
                    fn #fn_name(#comps_var: &mut #comps_type) -> Option<#ty> {
                        #body
                    }
                    fn #fn_name_vec(#comps_var: &mut #comps_type) -> Vec<#ty> {
                        match #fn_name(#comps_var) {
                            Some(t) => vec![t],
                            None => vec![]
                        }
                    }
                )
            }
            None => {
                let v = quote!(v);

                let mut arg_it = self.args.iter().enumerate();

                // Set initial vector
                let init = match arg_it.next() {
                    Some((_, arg)) => {
                        let var = component_var(arg.comp.idx);
                        if arg.is_mut {
                            quote!(#comps_var.#var.iter_mut().collect())
                        } else {
                            quote!(#comps_var.#var.iter().collect())
                        }
                    }
                    None => match &self.labels.as_ref().and_then(|cs| cs.first_true_symbol()) {
                        Some(comp) => {
                            let var = component_var(comp.idx);
                            quote!(#comps_var.#var.iter().collect())
                        }
                        None => quote!(#comps_var.#eids.iter().map(|k| (k, ())).collect()),
                    },
                };

                // Intersect vector with other components
                let intersects = arg_it.map_vec_into(|(i, arg)| {
                    let tmps = (0..i).map_vec_into(|i| format_ident!("v{i}"));
                    let tmp_n = format_ident!("v{}", i);
                    let closure = quote!(|(#(#tmps),*), #tmp_n| (#(#tmps,)* #tmp_n));
                    let var = component_var(arg.comp.idx);
                    if arg.is_mut {
                        quote!(#intersect_mut(#v, &mut #comps_var.#var, #closure))
                    } else {
                        quote!(#intersect(#v, &#comps_var.#var, #closure))
                    }
                });

                // Return statement, none means return empty vector
                let ret = match &self.labels {
                    None | Some(ComponentSetLabels::Constant(true)) => Some(quote!(
                        #v
                    )),
                    Some(ComponentSetLabels::Constant(false)) => None,
                    Some(ComponentSetLabels::Expression(e)) => Some(gen_filter_fn(v, e)),
                };

                let body = match ret {
                    Some(ret) => quote!(
                        let #v = #init;
                        #(let #v = #intersects;)*
                        #ret
                    ),
                    None => quote!(vec![]),
                };

                quote!(
                    fn #fn_name_vec(#comps_var: &mut #comps_type) -> Vec<#ty> {
                        #body
                    }
                )
            }
        }
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
