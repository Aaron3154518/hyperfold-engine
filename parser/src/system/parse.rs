use diagnostic::{CombineResults, ErrorSpan, ResultsTrait, ToErr};
use proc_macro2::Span;
use quote::ToTokens;
use syn::spanned::Spanned;

use shared::{
    parsing::SystemMacroArgs,
    syn::{
        error::{err, AddSpan, SpannedResult, SplitBuildResult},
        get_type_generics, parse_tokens, use_path_from_syn, ToRange,
    },
    traits::{Call, CollectVecInto, CombineOptions, PushInto, ToNone},
};

use crate::{
    parse::{
        resolve_path, resolve_syn_path, AstAttribute, AstFunction, DiscardSymbol, HardcodedSymbol,
        ItemPath, MatchSymbol, ModInfo,
    },
    resolve::Items,
};

#[derive(Clone, Debug)]
pub enum FnArgType {
    Event(usize),
    Global(usize),
    Entities { idx: usize, is_vec: bool },
}

impl std::fmt::Display for FnArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            match self {
                FnArgType::Event(i) => format!("Event {i}"),
                FnArgType::Global(i) => format!("Global {i}"),
                FnArgType::Entities { idx, is_vec } => format!(
                    "{} {idx}",
                    if *is_vec { "Vec<Entities>" } else { "Entities" }
                ),
            }
            .as_str(),
        )
    }
}

#[derive(Debug)]
pub struct FnArg {
    pub ty: FnArgType,
    pub is_mut: bool,
    pub ref_cnt: usize,
    pub span: Span,
}

impl FnArg {
    pub fn parse(
        attrs: &SystemMacroArgs,
        items: &Items,
        sig: &syn::Signature,
        (m, cr, crates): ModInfo,
    ) -> SpannedResult<Vec<Self>> {
        sig.inputs
            .iter()
            .map_vec_into(|arg| match arg {
                syn::FnArg::Receiver(r) => err("Cannot use self in function", r).as_err(),
                syn::FnArg::Typed(syn::PatType { ty, .. }) => {
                    FnArg::parse_type(ty, (m, cr, crates))
                }
            })
            .combine_results()
    }

    fn parse_type(ty: &syn::Type, (m, cr, crates): ModInfo) -> SpannedResult<Self> {
        let ty_str = ty.to_token_stream().to_string();
        match ty {
            syn::Type::Path(p) => {
                // TODO: Type alias support must include Vec (Add path resolution for built-ins)
                let path = use_path_from_syn(&m.path, &p.path);
                match path.join("::").as_str() {
                    "Vec" => match get_type_generics(p).as_ref().and_then(|v| v.first()) {
                        Some(syn::GenericArgument::Type(syn::Type::Path(ty))) => {
                            resolve_syn_path(&m.path, &ty.path, (m, cr, crates))
                                .expect_component_set()
                                .discard_symbol()
                                .add_span(ty)
                                .map(|idx| FnArgType::Entities { idx, is_vec: true })
                        }
                        _ => err("Invalid argument type", ty).as_err(),
                    },
                    _ => resolve_path(path, (m, cr, crates))
                        .add_span(ty)
                        .and_then(|sym| match sym.kind {
                            crate::parse::SymbolType::Global(g_sym) => {
                                Ok(FnArgType::Global(g_sym.idx))
                            }
                            crate::parse::SymbolType::Event(i) => Ok(FnArgType::Event(i)),
                            crate::parse::SymbolType::ComponentSet(idx) => {
                                Ok(FnArgType::Entities { idx, is_vec: false })
                            }
                            _ => err("Invalid argument type", ty).as_err(),
                        }),
                }
                .map(|data| Self {
                    ty: data,
                    is_mut: false,
                    ref_cnt: 0,
                    span: ty.span(),
                })
            }
            syn::Type::Reference(r) => {
                Self::parse_type(&r.elem, (m, cr, crates)).map(|mut fn_arg| {
                    fn_arg.ref_cnt += 1;
                    fn_arg.is_mut = fn_arg.is_mut || r.mutability.is_some();
                    fn_arg
                })
            }
            syn::Type::TraitObject(t) => {
                let traits = t
                    .bounds
                    .iter()
                    .filter_map(|tpb| match tpb {
                        syn::TypeParamBound::Trait(tr) => Some(tr),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                match traits.split_first() {
                    Some((tr, tail)) if tail.is_empty() => {
                        resolve_syn_path(&m.path, &tr.path, (m, cr, crates))
                            .expect_trait()
                            .discard_symbol()
                            .add_span(ty)
                            .map(|g_sym| Self {
                                ty: FnArgType::Global(g_sym.idx),
                                is_mut: false,
                                ref_cnt: 0,
                                span: ty.span(),
                            })
                    }
                    _ => err("Trait arguments must have only one trait type", ty).as_err(),
                }
            }
            _ => err("Invalid argument type", ty).as_err(),
        }
    }
}

impl std::fmt::Display for FnArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}{}{}",
            "&".repeat(self.ref_cnt),
            if self.is_mut { "mut " } else { "" },
            self.ty
        ))
    }
}

#[derive(Debug)]
pub struct ItemSystem {
    pub path: ItemPath,
    pub args: Vec<FnArg>,
    pub attr_args: SystemMacroArgs,
    pub file: String,
    pub span: Span,
    pub err_span: ErrorSpan,
}

impl ItemSystem {
    pub fn parse(
        fun: &AstFunction,
        attr: &AstAttribute,
        items: &Items,
        (m, cr, crates): ModInfo,
    ) -> SpannedResult<Self> {
        let path = m.path.to_vec().push_into(fun.sig.ident.to_string());
        parse_tokens(attr.args.clone()).and_then(|attr_args| {
            FnArg::parse(&attr_args, items, &fun.sig, (m, cr, crates)).map(|args| ItemSystem {
                path: ItemPath {
                    cr_idx: cr.idx,
                    path,
                },
                args,
                attr_args,
                file: m.get_file(),
                span: fun.sig.ident.span(),
                err_span: m.span.clone(),
            })
        })
    }
}
