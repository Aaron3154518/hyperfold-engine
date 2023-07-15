use quote::ToTokens;
use shared::{
    msg_result::{CombineMsgs, MsgResult},
    parsing::SystemMacroArgs,
    traits::{CollectVecInto, PushInto},
};

use crate::{
    parse::{
        AstAttribute, AstFunction, AstItem, DiscardSymbol, HardcodedSymbol, MatchSymbol, ModInfo,
    },
    resolve::{resolve_syn_path, ItemPath, Items, ResolveResultTrait},
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
                syn::FnArg::Receiver(_) => Err(vec!["Cannot use self in function".to_string()]),
                syn::FnArg::Typed(syn::PatType { ty, .. }) => {
                    FnArg::parse_type(ty, (m, cr, crates))
                }
            })
            .combine_msgs()
            .map_err(|mut errs| {
                errs.insert(
                    0,
                    format!(
                        "In system: '{}':",
                        m.path.to_vec().push_into(sig.ident.to_string()).join("::")
                    ),
                );
                errs
            })
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
                    .expect_symbol()
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
                                            .expect_symbol()
                                            .expect_component_set()
                                            .discard_symbol()
                                            .map(|idx| FnArgType::Entities { idx, is_vec: true })
                                    }
                                    _ => Err(vec![format!(
                                        "Invalid argument type: {}",
                                        sym.path.join("::")
                                    )]),
                                }
                            }
                            _ => Err(vec![format!(
                                "Invalid argument type: {}",
                                sym.path.join("::")
                            )]),
                        },
                        _ => Err(vec![format!(
                            "Invalid argument type: {}",
                            sym.path.join("::")
                        )]),
                    })
                    .map(|data| Self {
                        ty: data,
                        is_mut: false,
                        ref_cnt: 0,
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
                    Err(vec![format!(
                        "Trait arguments may only have one trait type: {ty_str}"
                    )])
                } else {
                    resolve_syn_path(&m.path, &traits[0].path, (m, cr, crates))
                        .expect_symbol()
                        .expect_trait()
                        .discard_symbol()
                        .map(|g_sym| Self {
                            ty: FnArgType::Global(g_sym.idx),
                            is_mut: false,
                            ref_cnt: 0,
                        })
                }
            }
            _ => Err(vec![format!("Invalid argument type: {ty_str}")]),
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
        })
    }
}
