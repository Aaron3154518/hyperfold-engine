use proc_macro2::TokenStream;
use shared::util::FindFrom;
use syn::{spanned::Spanned, Error, Token};

use super::ast_resolve::Path;

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
    ($tokens: ident, $token: ident, $msg: literal) => {
        match $tokens.parse() {
            Ok(t) => t,
            Err(e) => return err!($token, $msg, e),
        }
    };

    ($tokens: ident, $type: ty, $token: ident, $msg: literal) => {
        match $tokens.parse::<$type>() {
            Ok(t) => t,
            Err(e) => return err!($token, $msg, e),
        }
    };
}

#[derive(Copy, Clone, Debug)]
pub enum LabelOp {
    And,
    Or,
}

impl syn::parse::Parse for LabelOp {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input
            .parse::<Token!(&&)>()
            .map(|_| Self::And)
            .or_else(|_| input.parse::<Token!(||)>().map(|_| Self::Or))
    }
}

#[derive(Clone, Debug)]
pub enum LabelItem {
    Parens {
        not: bool,
        body: Box<LabelItem>,
    },
    Item {
        not: bool,
        ty: Vec<String>,
    },
    Expression {
        lhs: Box<LabelItem>,
        op: LabelOp,
        rhs: Box<LabelItem>,
    },
}

impl syn::parse::Parse for LabelItem {
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
                        _ => err!(ty, "Expected parentheses or ident in label"),
                    },
                )
            },
            |g| {
                syn::parse2::<Expression>(g.stream()).map(|e| Self::Parens {
                    not,
                    body: Box::new(e.split()),
                })
            },
        )
    }
}

#[derive(Clone, Debug)]
pub struct Expression {
    noops: Vec<LabelItem>,
    ops: Vec<LabelOp>,
}

impl Expression {
    pub fn split(&self) -> LabelItem {
        self.split_impl(0, self.ops.len())
    }

    fn split_impl(&self, lb: usize, ub: usize) -> LabelItem {
        match self
            .ops
            .find_from_to(|op| matches!(op, LabelOp::And), lb, ub)
            .or_else(|| {
                self.ops
                    .find_from_to(|op| matches!(op, LabelOp::Or), lb, ub)
            }) {
            Some(i) => LabelItem::Expression {
                lhs: Box::new(self.split_impl(lb, i)),
                op: self.ops[i],
                rhs: Box::new(self.split_impl(i + 1, ub)),
            },
            None => self.noops[lb].clone(),
        }
    }
}

impl syn::parse::Parse for Expression {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let body = match input.parse() {
            Ok(t) => t,
            Err(e) => return err!(input, "Expected label expression in parentheses", e),
        };
        let mut noops = vec![body];
        let mut ops = Vec::new();
        while let Ok(op) = input.parse() {
            ops.push(op);
            noops.push(match input.parse() {
                Ok(t) => t,
                Err(e) => return err!(input, "Expected NoOp after Op", e),
            });
        }
        Ok(Self { noops, ops })
    }
}

#[derive(Debug)]
pub struct ComponentSetItem {
    var: String,
    ty: Vec<String>,
    ref_cnt: usize,
    is_mut: bool,
}

impl ComponentSetItem {
    pub fn from(var: String, ty: syn::Type) -> syn::Result<Self> {
        let span = ty.span();
        return match ty {
            syn::Type::Path(ty) => Ok(Self {
                var,
                ty: ty
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect(),
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

impl syn::parse::Parse for ComponentSetItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // var : type
        let var = match input.parse::<syn::Ident>() {
            Ok(i) => i.to_string(),
            Err(e) => return err!(input, "Expected variable name", e),
        };
        if let Err(e) = input.parse::<syn::token::Colon>() {
            return err!(input, "Expected ':'", e);
        }
        ComponentSetItem::from(
            var,
            parse_expect!(input, syn::Type, input, "Expected variable type"),
        )
    }
}

#[derive(Debug)]
pub struct ComponentSet {
    pub path: Path,
    pub args: Vec<ComponentSetItem>,
    pub labels: Option<LabelItem>,
}

impl syn::parse::Parse for ComponentSet {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut _comma: syn::token::Comma;

        let mut labels = None;

        // First ident
        let mut first_ident = parse_expect!(input, syn::Ident, input, "Expected ident");
        if first_ident == "labels" {
            // Labels
            labels = match input.parse::<proc_macro2::Group>() {
                Ok(g) => {
                    let stream = g.stream();
                    if stream.is_empty() {
                        None
                    } else {
                        Some(match syn::parse2::<Expression>(stream) {
                            Ok(e) => e.split(),
                            Err(e) => return err!(g, "Failed to parse labels", e),
                        })
                    }
                }
                Err(e) => return err!(input, "Expected parentheses after labels", e),
            };
            _comma = parse_expect!(input, input, "Expected comma");
            first_ident = parse_expect!(input, input, "Expected ident");
        }

        let path = Path {
            cr_idx: 0,
            path: vec![first_ident.to_string()],
        };

        let mut args = Vec::new();
        while let Result::Ok(_) = input.parse::<syn::token::Comma>() {
            args.push(parse_expect!(
                input,
                input,
                "Expected argument variable-tyle pair"
            ));
        }

        Ok(Self { path, args, labels })
    }
}

impl ComponentSet {
    pub fn parse(cr_idx: usize, path: Vec<String>, ts: TokenStream) -> syn::Result<Self> {
        syn::parse2::<Self>(ts).map(|mut cs| {
            cs.path = Path {
                cr_idx,
                path: [path, cs.path.path].concat(),
            };
            cs
        })
    }
}
