use std::{fmt::Display, path::PathBuf};

use quote::ToTokens;
use shared::util::{Catch, JoinMap};

use crate::{
    parse::{ast_crate::AstCrate, ast_mod::AstMod},
    resolve::{
        path::{resolve_path, ItemPath},
        paths::{EnginePaths, Paths},
    },
    util::parse_syn_path,
};

#[derive(Clone, Debug)]
pub enum FnArgType {
    Path(ItemPath),
    Trait(ItemPath),
    Entities(ItemPath),
}

impl std::fmt::Display for FnArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = match self {
            FnArgType::Path(p) | FnArgType::Trait(p) | FnArgType::Entities(p) => p.path.join("::"),
        };

        f.write_fmt(format_args!(
            "{}",
            match self {
                FnArgType::Path(_) => path,
                FnArgType::Trait(_) => format!("dyn {path}"),
                FnArgType::Entities(_) => format!("Vec<{path}>"),
            }
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FnArg {
    pub ty: FnArgType,
    pub is_mut: bool,
    pub ref_cnt: usize,
}

impl FnArg {
    pub fn from(
        ty: &syn::PatType,
        cr: &AstCrate,
        m: &AstMod,
        paths: &Paths,
        crates: &Vec<AstCrate>,
    ) -> Self {
        Self::parse_type(&ty.ty, cr, m, paths, crates)
    }

    fn parse_type(
        ty: &syn::Type,
        cr: &AstCrate,
        m: &AstMod,
        paths: &Paths,
        crates: &Vec<AstCrate>,
    ) -> Self {
        let resolve_syn_path = |path: &syn::Path| {
            let path = parse_syn_path(&m.path, path);
            let path_str = path.join("::");
            resolve_path(path, cr, m, crates).catch(format!(
                "Could not find argument type: \"{}\" in crate {}",
                path_str, cr.idx
            ))
        };

        let ty_str = ty.to_token_stream().to_string();
        match ty {
            syn::Type::Path(p) => {
                let generics = p.path.segments.last().and_then(|s| match &s.arguments {
                    syn::PathArguments::AngleBracketed(ab) => {
                        Some(ab.args.iter().collect::<Vec<_>>())
                    }
                    _ => None,
                });

                let path = resolve_syn_path(&p.path);

                let ty = if &path == paths.get_engine_path(EnginePaths::Entities) {
                    match generics.as_ref().map(|args| &args[..]) {
                        Some([syn::GenericArgument::Type(syn::Type::Path(p))]) => {
                            FnArgType::Entities(resolve_syn_path(&p.path))
                        }
                        v => {
                            panic!(
                                "Expected single non-ref type path inside Vec<>, found {}",
                                v.map_or(String::new(), |args| args
                                    .join_map(|arg| arg.to_token_stream().to_string(), ","))
                            )
                        }
                    }
                } else {
                    FnArgType::Path(path)
                };

                Self {
                    ty,
                    is_mut: false,
                    ref_cnt: 0,
                }
            }
            syn::Type::Reference(r) => {
                let mut fn_arg = Self::parse_type(&r.elem, cr, m, paths, crates);
                fn_arg.ref_cnt += 1;
                fn_arg.is_mut = fn_arg.is_mut || r.mutability.is_some();
                fn_arg
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
                    panic!("Trait arguments may only have one trait type: {ty_str}");
                }
                Self {
                    ty: FnArgType::Trait(resolve_syn_path(&traits[0].path)),
                    is_mut: false,
                    ref_cnt: 0,
                }
            }
            _ => panic!("Invalid argument type: {ty_str}"),
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
