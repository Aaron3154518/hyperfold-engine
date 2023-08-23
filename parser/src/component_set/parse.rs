use diagnostic::{CatchErr, ToErr};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    spanned::Spanned,
    token::{Colon, Comma},
    Error, Token,
};

use crate::parse::ItemPath;

use shared::{
    syn::{
        error::{CatchSynError, CriticalResult, ToError},
        get_type_generics, parse_tokens, use_path_from_syn, Parse, StreamParse,
    },
    traits::{CollectVec, PushInto},
};

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

impl Parse for LabelOp {
    fn parse(input: syn::parse::ParseStream) -> CriticalResult<Self> {
        let span = input.span();
        input
            .parse::<Token!(&&)>()
            .map(|_| Self::And)
            .or_else(|_| input.parse::<Token!(||)>().map(|_| Self::Or))
            .catch_err(span.error("Expected Op: '||' or '&&'"))
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
        span: Span,
    },
}

impl Expression {
    pub fn to_item(self, neg: bool) -> CriticalResult<AstLabelItem> {
        let mut item = match self {
            Expression::Item(i) => i,
            Expression::Expr {
                first,
                noops,
                ops,
                span,
            } => {
                let mut get_item =
                    |items: Vec<AstLabelItem>, op: LabelOp| -> CriticalResult<AstLabelItem> {
                        Ok(if items.len() <= 1 {
                            items
                                .into_iter()
                                .next()
                                .ok_or(span.error("Empty label expression").as_vec())?
                        } else {
                            // Combine expressions with the same operation
                            let items = items.into_iter().fold(Vec::new(), |mut items, item| {
                                match item {
                                    AstLabelItem::Item { not, ty, span } => {
                                        items.push(AstLabelItem::Item { not, ty, span })
                                    }
                                    AstLabelItem::Expression {
                                        op: exp_op,
                                        items: exp_items,
                                        span,
                                    } => {
                                        if op == exp_op {
                                            items.extend(exp_items);
                                        } else {
                                            items.push(AstLabelItem::Expression {
                                                op: exp_op,
                                                items: exp_items,
                                                span,
                                            });
                                        }
                                    }
                                }
                                items
                            });
                            AstLabelItem::Expression { op, items, span }
                        })
                    };
                let mut ors = Vec::new();
                let mut ands = vec![first];
                for (item, op) in noops.into_iter().zip(ops.into_iter()) {
                    match op {
                        LabelOp::And => ands.push(item),
                        LabelOp::Or => {
                            ors.push(get_item(ands, LabelOp::And)?);
                            ands = vec![item];
                        }
                    }
                }
                ors.push(get_item(ands, LabelOp::And)?);
                get_item(ors, LabelOp::Or)?
            }
        };
        if neg {
            item.negate();
        }
        Ok(item)
    }
}

impl Parse for Expression {
    fn parse(input: syn::parse::ParseStream) -> CriticalResult<Self> {
        let span = input.span();

        let first = input.parse_stream()?;

        let mut noops = vec![];
        let mut ops = vec![];

        while !input.is_empty() {
            ops.push(input.parse_stream()?);
            noops.push(input.parse_stream()?);
        }

        Ok(match ops.is_empty() {
            true => Self::Item(first),
            false => Self::Expr {
                first,
                noops,
                ops,
                span,
            },
        })
    }
}

// Flattened label set
#[derive(Clone, Debug)]
pub enum AstLabelItem {
    Item {
        not: bool,
        ty: Vec<String>,
        span: Span,
    },
    Expression {
        op: LabelOp,
        items: Vec<AstLabelItem>,
        span: Span,
    },
}

impl AstLabelItem {
    pub fn negate(&mut self) {
        match self {
            AstLabelItem::Item { not, .. } => *not = !*not,
            AstLabelItem::Expression { op, items, .. } => {
                op.negate();
                items.iter_mut().for_each(|i| i.negate());
            }
        }
    }

    pub fn span(&self) -> &Span {
        match self {
            AstLabelItem::Item { span, .. } | AstLabelItem::Expression { span, .. } => span,
        }
    }
}

impl Parse for AstLabelItem {
    fn parse(input: syn::parse::ParseStream) -> CriticalResult<Self> {
        let mut not = false;
        while input.parse::<Token!(!)>().is_ok() {
            not = !not;
        }

        let span = input.span();

        input.parse::<proc_macro2::Group>().map_or_else(
            |_| {
                input
                    .parse::<syn::TypePath>()
                    .catch_err(span.error("Expected label type"))
                    .map(|p| Self::Item {
                        not,
                        span: p.span(),
                        ty: p
                            .path
                            .segments
                            .iter()
                            .map(|s| s.ident.to_string())
                            .collect(),
                    })
            },
            |g| parse_tokens::<Expression>(g.stream()).and_then(|e| e.to_item(not)),
        )
    }
}

impl std::fmt::Display for AstLabelItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AstLabelItem::Item { not, ty, .. } => f.write_fmt(format_args!(
                "{}{}",
                if *not { "!" } else { "" },
                ty.join("::")
            )),
            AstLabelItem::Expression { op, items, .. } => f.write_fmt(format_args!(
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
    pub var: String,
    pub ty: ItemPath,
    pub ref_cnt: usize,
    pub is_mut: bool,
    pub is_opt: bool,
    pub span: Span,
}

impl AstComponentSetItem {
    pub fn from(var: String, ty: &syn::Type) -> CriticalResult<Self> {
        let span = ty.span();
        match ty {
            syn::Type::Path(ty) => {
                let mut path = use_path_from_syn(&Vec::new(), &ty.path);
                match path.join("::").as_str() {
                    "Option" => {
                        match get_type_generics(&ty)
                            .ok_or(ty.error("Missing generics after Option<>").as_vec())?[..]
                        {
                            [arg] => match arg {
                                syn::GenericArgument::Type(ty) => {
                                    let mut i = Self::from(var, ty)?;
                                    if i.is_opt {
                                        return ty.error("Cannot nest Option<>").as_err();
                                    }
                                    i.is_opt = true;
                                    Ok(i)
                                }
                                _ => arg.error("Expected type generic").as_err(),
                            },
                            _ => ty.error("Multiple generics after Option<>").as_err(),
                        }
                    }
                    _ => Ok(Self {
                        var,
                        ty: ItemPath { cr_idx: 0, path },
                        ref_cnt: 0,
                        is_mut: false,
                        is_opt: false,
                        span: ty.span(),
                    }),
                }
            }
            syn::Type::Reference(ty) => Self::from(var, &ty.elem).and_then(|mut i| {
                if i.is_opt {
                    return ty
                        .error("Optional arguments may not be taken by reference")
                        .as_err();
                }
                i.ref_cnt += 1;
                i.is_mut |= ty.mutability.is_some();
                Ok(i)
            }),
            _ => ty.error("Invalid argument type").as_err(),
        }
    }
}

impl Parse for AstComponentSetItem {
    fn parse(input: syn::parse::ParseStream) -> CriticalResult<Self> {
        let span = input.span();
        // var : type
        let var = input
            .parse::<syn::Ident>()
            .map(|i| i.to_string())
            .catch_syn_err("Expected variable name")?;
        input.parse::<Colon>().catch_syn_err("Expected ':'")?;
        AstComponentSetItem::from(var, &input.parse().catch_syn_err("Expected variable type")?)
    }
}

#[derive(Debug)]
pub struct AstComponentSet {
    pub ident: syn::Ident,
    pub args: Vec<AstComponentSetItem>,
    pub labels: Option<AstLabelItem>,
}

impl Parse for AstComponentSet {
    fn parse(input: syn::parse::ParseStream) -> CriticalResult<Self> {
        let mut labels = None;

        // First ident
        let mut first_ident: syn::Ident = input.parse().catch_syn_err("Expected ident")?;
        if first_ident == "labels" {
            // Labels
            labels = input
                .parse::<proc_macro2::Group>()
                .catch_err(first_ident.error("Expected parentheses after labels"))
                .and_then(|g| {
                    let stream = g.stream();
                    Ok(match stream.is_empty() {
                        true => None,
                        false => Some(
                            parse_tokens::<Expression>(stream).and_then(|e| e.to_item(false))?,
                        ),
                    })
                })?;
            input
                .parse::<Comma>()
                .catch_syn_err("Expected ',' after label expression")?;
            first_ident = input
                .parse()
                .catch_syn_err("Expected ident after label expression")?;
        }

        let mut args = Vec::new();
        while input.parse::<Comma>().is_ok() {
            if input.is_empty() {
                break;
            }
            args.push(input.parse_stream()?);
        }

        Ok(Self {
            ident: first_ident,
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
