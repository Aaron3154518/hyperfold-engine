use std::{fmt::Display, path::PathBuf};

use quote::ToTokens;
use shared::util::Catch;

use crate::{
    resolve::ast_resolve::{resolve_path, Path},
    util::parse_syn_path,
};

use super::{ast_crate::Crate, ast_mod::Mod};

#[derive(Clone, Debug)]
pub struct FnArg {
    pub ty: syn::Path,
    pub is_mut: bool,
    pub is_trait: bool,
    pub ref_cnt: usize,
}

impl FnArg {
    pub fn from(ty: syn::PatType) -> Self {
        Self::parse_type(ty)
    }

    fn parse_type(&mut self, ty: syn::Type) -> Self {
        let ty_str = ty.to_token_stream().to_string();
        match ty {
            syn::Type::Path(p) => Self {
                ty: p.path,
                is_mut: false,
                is_dyn: false,
                ref_cnt: 0,
            }
            syn::Type::Reference(r) => {
                let mut fn_arg = self.parse_type(cr_idx, super_path, &r.elem);
                fn_arg.ref_cnt += 1;
                fn_arg.is_mut = fn_arg.is_mut || r.mutability.is_some();
                fn_arg
            }
            syn::Type::TraitObject(t) => {
                let traits = t.bounds.into_iter().filter_map(
                    |tpb| match tbp {
                        syn::TypeParamBound::Trait(tr) => Some(tr),
                        _ => None
                    }
                ).collect::<Vec<_>>();
                if traits.len() != 1 {
                    panic!("Trait arguments may only have one trait type: {ty_str}");
                }
                Self {
                    ty: traits[0].path,
                    is_mut: false,
                    is_trait: true,
                    ref_cnt: 0
                }
            }
            _ => panic!("Invalid argument type: {ty_str}"),
        }
    }

    pub fn resolve_paths(&mut self, cr: &Crate, m: &Mod, crates: &Vec<Crate>) {
        // Resolve all paths
        let p = match &mut self.ty {
            FnArgType::Path(p) | FnArgType::Trait(p) | FnArgType::Vec(p) => p,
        };
        *p = resolve_path(p.path.to_vec(), cr, m, crates).catch(format!(
            "Could not find argument type: \"{}\" in crate {}",
            p.path.join("::"),
            p.cr_idx
        ));
    }
}

impl std::fmt::Display for FnArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "{}{}{}{}",
                if self.ident.is_empty() {
                    String::new()
                } else {
                    format!("{}: ", self.ident)
                },
                "&".repeat(self.ref_cnt),
                if self.mutable { "mut " } else { "" },
                self.ty
            )
            .as_str(),
        )
    }
}

#[derive(Debug)]
pub enum FnArgType {
    
}
