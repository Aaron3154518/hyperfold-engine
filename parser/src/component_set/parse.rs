use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, Error, Token};

use crate::{err, parse::ItemPath, parse_expect};

use shared::traits::CollectVec;

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
pub enum AstLabelItem {
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
pub struct AstComponentSetItem {
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
pub struct AstComponentSet {
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
