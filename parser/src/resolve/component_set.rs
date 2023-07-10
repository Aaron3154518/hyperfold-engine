use std::{collections::VecDeque, fmt::Display, ops::Neg};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use shared::util::{Call, Catch, FindFrom, Get, JoinMap, JoinMapInto, MapNone, NoneOr, PushInto};
use syn::{spanned::Spanned, Error, Token};

use crate::{
    codegen2::{util::Quote, CodegenIdents, CODEGEN_IDENTS},
    parse::{
        AstCrate, ComponentSymbol, DiscardSymbol, MatchSymbol, ModInfo, SymbolType,
        {AstMod, Symbol},
    },
    resolve::util::{get_mut, CombineMsgs, MsgsResult},
    resolve::{path::resolve_path, util::get_fn_name},
};

use super::{
    constants::{component_set_keys_fn, component_set_var, component_var},
    function_arg::ComponentSetFnArg,
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
                .map(|comp| Self::Item { not, comp: comp }),
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
}

impl ComponentSet {
    // Gets first arg, precedence given to singleton args
    pub fn first_arg(&self) -> Option<&ComponentSetItem> {
        self.args
            .iter()
            .find(|item| item.comp.args.is_singleton)
            .or(self.args.first())
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

    // Gets first arg or required true label that is singleton
    pub fn first_singleton(&self) -> Option<&ComponentSymbol> {
        self.first_arg()
            .and_then(|arg| arg.comp.args.is_singleton.then_some(&arg.comp))
            .or_else(|| {
                self.first_label()
                    .and_then(|comp| comp.args.is_singleton.then_some(comp))
            })
    }

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
                    labels,
                    args,
                }
            })
    }

    // Generates function which get the keys used in a component set
    pub fn quote(
        &self,
        cs_idx: usize,
        filter: &syn::Path,
        entity: &syn::Path,
        entity_set: &syn::Path,
    ) -> TokenStream {
        // Remove duplicate arguments
        let mut args = self.args.to_vec();
        args.sort_by_key(|item| item.comp.idx);
        args.dedup_by_key(|item| item.comp.idx);

        // Get first arg, priority to singleton
        let singleton_arg = args.iter().find(|item| item.comp.args.is_singleton);
        let first_arg = singleton_arg.or(args.first());

        // Generate final function signatures in case we return early
        let CodegenIdents {
            components,
            eids_var,
            comps_var,
            ..
        } = &*CODEGEN_IDENTS;

        let get_keys = component_set_keys_fn(cs_idx, false);
        let get_keys_fn = quote!(#get_keys<'a>(#comps_var: &#components, #eids_var: &'a #entity_set) -> Option<&'a #entity>);
        let get_keys_vec = component_set_keys_fn(cs_idx, true);
        let get_keys_vec_fn = quote!(#get_keys_vec<'a>(#comps_var: &#components, #eids_var: &'a #entity_set) -> Vec<(&'a #entity, ())>);

        // Get labels expression and first/singleton label
        let mut first_label = None;
        let mut singleton_label = None;
        let labels = match &self.labels {
            Some(labels) => match labels {
                // Label expression is always true, ignore labels
                ComponentSetLabels::Constant(true) => None,
                // Label expression is always false, skip computations
                ComponentSetLabels::Constant(false) => {
                    let fn_vec = quote!(fn #get_keys_vec_fn { vec![] });
                    return match singleton_arg {
                        Some(_) => quote!(
                            fn #get_keys_fn { None }
                            #fn_vec
                        ),
                        None => fn_vec,
                    };
                }
                ComponentSetLabels::Expression(expr) => {
                    singleton_label = expr.true_symbols.iter().find(|comp| comp.args.is_singleton);
                    first_label = singleton_label.clone().or(expr.true_symbols.first());
                    Some(expr)
                }
            },
            None => None,
        };

        // Create statement to initialize the result
        // Returns
        //   Has Singleton ? Option<&'a K> : Vec<(&'a K, ())>
        //   Where 'a is bound to #eids
        let init = match (first_arg, first_label) {
            // Arg or label
            (Some(ComponentSetItem { comp, .. }), _) | (_, Some(comp)) => {
                let var = component_var(comp.idx);
                match comp.args.is_singleton {
                    // Option<K>
                    true => {
                        quote!(#comps_var.#var.get_key().and_then(|k| #comps_var.#eids_var.get(k)))
                    }
                    // Vec<(K, V)>
                    false => {
                        quote!(#comps_var.#var.keys().filter_map(|k| #eids_var.get(k).map(|k| (k, ()))).collect())
                    }
                }
            }
            // No arg and no label
            (None, None) => {
                // Vec<(K, V)>
                quote!(#eids_var.iter().map(|k| (k, ())).collect())
            }
        };

        // Add filter if labels are present
        let cs = component_set_var(cs_idx);
        let init_and_filter = match &labels {
            Some(expr) => {
                let f = quote!(f);
                let label_expr = expr.labels.quote(&|comp| component_var(comp.idx));
                let label_vars = expr
                    .iter_symbols()
                    .map_vec_into(|comp| component_var(comp.idx));
                let label_fn = quote!(|[#(#label_vars),*]| #label_expr);
                let eval_labels = quote!(#f([#(&#comps_var.#label_vars.contains_key(k)),*]));
                match (first_arg, first_label) {
                    // Option<K>
                    (Some(ComponentSetItem { comp, .. }), _) | (_, Some(comp))
                        if comp.args.is_singleton =>
                    {
                        quote!(
                            let #cs = #init;
                            let #f = #label_fn;
                            let #cs = #cs.and_then(|k| #eval_labels.then_some(k));
                        )
                    }
                    // Vec<(K, V)>
                    _ => quote!(
                        let mut #cs = #init;
                        let #f = #label_fn;
                        #cs.drain_filter(|(k, _)| !#eval_labels);
                    ),
                }
            }
            None => quote!(let #cs = #init;),
        };

        // Generate final functions
        match (first_arg, first_label) {
            // Option<K>
            (Some(ComponentSetItem { comp, .. }), _) | (_, Some(comp))
                if comp.args.is_singleton =>
            {
                quote!(
                    fn #get_keys_fn {
                        #init_and_filter
                        #cs
                    }
                    fn #get_keys_vec_fn {
                        Self::#get_keys(#comps_var, #eids_var).map_or(vec![], |t| vec![t])
                    }
                )
            }
            // Vec<K>
            _ => {
                quote!(
                    fn #get_keys_vec_fn {
                        #init_and_filter
                        #cs
                    }
                )
            }
        }
    }

    // Generates inline code to construct values from keys
    pub fn quote_build(&self, v: syn::Ident, ty: &syn::Path, intersect: &syn::Path) -> TokenStream {
        // Remove duplicate arguments
        let mut args = self.args.to_vec();
        args.sort_by_key(|item| item.comp.idx);
        args.dedup_by_key(|item| item.comp.idx);

        let (first_arg, first_label) = (self.first_arg(), self.first_label());

        let CodegenIdents {
            eid_var, comps_var, ..
        } = &*CODEGEN_IDENTS;

        // Unique component variables
        let var = args.map_vec(|item| component_var(item.comp.idx));
        // Full list of component names and variables
        let arg_name = self.args.map_vec(|item| format_ident!("{}", item.var));
        let arg_var = self.args.map_vec(|item| component_var(item.comp.idx));
        match (first_arg, first_label) {
            // Option<K>, No args
            (None, Some(comp)) if comp.args.is_singleton => {
                quote!(let #v = #v.map(|k| #ty { #eid_var: k });)
            }
            // Option<K>
            (Some(ComponentSetItem { comp, .. }), _) | (_, Some(comp))
                if comp.args.is_singleton =>
            {
                let get = args.map_vec(|item| get_fn_name("get", item.is_mut));
                quote!(
                    let #v = #v.and_then(|k| match (#(#comps_var.#var.#get(k)),*) {
                        (#(Some(#var)),*) => Some(#ty {
                            #eid_var: k,
                            #(#arg_name: #arg_var),*
                        }),
                        _ => None
                    });
                )
            }
            // Vec<(K, V)>, No args
            (None, _) => {
                quote!(
                    let #v = #v.into_iter()
                        .map(|(k, _)| #ty { eid: k })
                        .collect();
                )
            }
            // Vec<(K, V)>
            _ => {
                let value_fn = (0..args.len()).map_vec_into(|i| match i {
                    0 => quote!(|_, v| v),
                    i => {
                        let tmps = (0..i).map_vec_into(|i| format_ident!("v{i}"));
                        let tmp_n = format_ident!("v{}", i);
                        quote!(|(#(#tmps),*), #tmp_n| (#(#tmps,)* #tmp_n))
                    }
                });
                let iter = args.map_vec(|item| get_fn_name("iter", item.is_mut));
                quote!(
                    #(let #v = #intersect(#v, #comps_var.#var.#iter(), #value_fn);)*
                    let #v = #v.into_iter()
                        .map(|k, (#(#var),*)| #ty {
                            #eid_var: k,
                            #(#arg_name: #arg_var),*
                        })
                        .collect();
                )
            }
        }
    }

    // Returns (code to build cs's, Vec<expression to pass each cs into a function>, Vec<singleton cs vars>)
    pub fn quote_args(
        component_sets: &Vec<(ComponentSetFnArg, &Self, syn::Path)>,
        intersect: &syn::Path,
    ) -> (TokenStream, Vec<TokenStream>, Vec<TokenStream>) {
        let CodegenIdents {
            eids_var,
            comps_var,
            ..
        } = &*CODEGEN_IDENTS;

        let mut c_sets = component_sets.iter().collect::<Vec<_>>();
        c_sets.sort_by_key(|(arg, ..)| arg.idx);
        c_sets.dedup_by_key(|(arg, ..)| arg.idx);

        // Get all the keys
        // Now we have immutable references to eids
        let (var, get_keys_fn) = c_sets.unzip_vec(|(arg, ..)| {
            (
                component_set_var(arg.idx),
                component_set_keys_fn(arg.idx, arg.is_vec),
            )
        });

        // Build sets
        let build = c_sets
            .iter()
            .zip(var.iter())
            .map_vec_into(|((_, cs, ty), v)| cs.quote_build(v.clone(), ty, intersect));

        (
            quote!(
                #(let #var = Self::#get_keys_fn(#comps_var, &#comps_var.#eids_var);)*
                #(#build)*
            ),
            component_sets.map_vec(|(arg, ..)| {
                let var = component_set_var(arg.idx);
                // Get last instance of the component set arg
                match component_sets
                    .iter()
                    .rev()
                    .find(|(arg2, ..)| arg.idx == arg2.idx)
                {
                    // If we aren't the last instance, pass a clone
                    Some((arg2, ..)) if arg2.arg_idx != arg.arg_idx => quote!(#var.clone()),
                    _ => var.quote(),
                }
            }),
            component_sets.filter_map_vec(|(arg, cs, _)| match cs.first_singleton() {
                Some(_) => Some(component_set_var(arg.idx).quote()),
                None => None,
            }),
        )
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
