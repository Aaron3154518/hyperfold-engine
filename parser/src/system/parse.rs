use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};
use proc_macro2::Span;
use quote::ToTokens;
use syn::spanned::Spanned;

use shared::{
    msg_result::CombineMsgs,
    parsing::SystemMacroArgs,
    traits::{CollectVecInto, PushInto},
};

use crate::{
    parse::{
        resolve_syn_path, AstAttribute, AstFunction, AstItem, DiscardSymbol, HardcodedSymbol,
        ItemPath, MatchSymbol, ModInfo,
    },
    resolve::Items,
    utils::{syn::ToRange, InjectSpan, Msg, MsgResult},
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
    ) -> MsgResult<Vec<Self>> {
        sig.inputs
            .iter()
            .map_vec_into(|arg| match arg {
                syn::FnArg::Receiver(r) => {
                    Err(vec![Msg::for_mod("Cannot use self in function", m, r)])
                }
                syn::FnArg::Typed(syn::PatType { ty, .. }) => {
                    FnArg::parse_type(ty, (m, cr, crates))
                }
            })
            .combine_msgs()
    }

    fn parse_type(ty: &syn::Type, (m, cr, crates): ModInfo) -> MsgResult<Self> {
        let ty_str = ty.to_token_stream().to_string();
        match ty {
            syn::Type::Path(p) => {
                let generics = p.path.segments.last().and_then(|s| match &s.arguments {
                    syn::PathArguments::AngleBracketed(ab) => {
                        Some(ab.args.iter().collect::<Vec<_>>())
                    }
                    _ => None,
                });

                resolve_syn_path(&m.path, &p.path, (m, cr, crates))
                    .in_mod(m, &ty)
                    .and_then(|sym| match sym.kind {
                        crate::parse::SymbolType::Global(g_sym) => Ok(FnArgType::Global(g_sym.idx)),
                        crate::parse::SymbolType::Event(i) => Ok(FnArgType::Event(i)),
                        crate::parse::SymbolType::ComponentSet(idx) => {
                            Ok(FnArgType::Entities { idx, is_vec: false })
                        }
                        crate::parse::SymbolType::Hardcoded(s) => match s {
                            HardcodedSymbol::Entities => {
                                match generics.as_ref().and_then(|v| v.first()) {
                                    Some(syn::GenericArgument::Type(syn::Type::Path(ty))) => {
                                        resolve_syn_path(&m.path, &ty.path, (m, cr, crates))
                                            .expect_component_set_in_mod(m, ty)
                                            .discard_symbol()
                                            .map(|idx| FnArgType::Entities { idx, is_vec: true })
                                    }
                                    _ => Err(vec![Msg::for_mod("Invalid argument type", m, ty)]),
                                }
                            }
                            _ => Err(vec![Msg::for_mod("Invalid argument type", m, ty)]),
                        },
                        _ => Err(vec![Msg::for_mod("Invalid argument type", m, ty)]),
                    })
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
                if traits.len() != 1 {
                    Err(vec![Msg::for_mod(
                        "Trait arguments may only have one trait type",
                        m,
                        ty,
                    )])
                } else {
                    resolve_syn_path(&m.path, &traits[0].path, (m, cr, crates))
                        .expect_trait_in_mod(m, ty)
                        .discard_symbol()
                        .map(|g_sym| Self {
                            ty: FnArgType::Global(g_sym.idx),
                            is_mut: false,
                            ref_cnt: 0,
                            span: ty.span(),
                        })
                }
            }
            _ => Err(vec![Msg::for_mod("Invalid argument type", m, ty)]),
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
    pub file: usize,
    pub span: Span,
    pub span_start: Option<usize>,
}

impl ItemSystem {
    pub fn parse(
        fun: &AstItem<AstFunction>,
        attr: &AstAttribute,
        items: &Items,
        (m, cr, crates): ModInfo,
    ) -> MsgResult<Self> {
        let path = m.path.to_vec().push_into(fun.data.sig.ident.to_string());
        let attr_args = SystemMacroArgs::from(&attr.args);
        FnArg::parse(&attr_args, items, &fun.data.sig, (m, cr, crates)).map(|args| ItemSystem {
            path: ItemPath {
                cr_idx: cr.idx,
                path,
            },
            args,
            attr_args,
            file: m.span_file,
            span: fun.data.sig.ident.span(),
            span_start: m.span_start,
        })
    }

    pub fn new_msg(&self, msg: &str, span: &impl Spanned) -> Msg {
        Msg::for_file(msg, self.file, span, self.span_start)
    }

    pub fn msg(&self, msg: &str) -> Msg {
        Msg::for_file(msg, self.file, &self.span, self.span_start)
    }
}
