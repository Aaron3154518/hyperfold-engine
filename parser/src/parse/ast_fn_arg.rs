use std::{fmt::Display, path::PathBuf};

use quote::ToTokens;
use shared::util::Catch;

use crate::{
    resolve::ast_resolve::{resolve_path, Path},
    util::parse_syn_path,
};

use super::{ast_crate::Crate, ast_mod::Mod};

// Possible function arg types: either a type or a Vec<(ty1, ty2, ...)>
#[derive(Clone, Debug)]
pub enum FnArgType {
    // Type
    Path(Path),
    // &dyn Type
    Trait(Path),
    // Vec<Type>
    Vec(Path),
}

impl std::fmt::Display for FnArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FnArgType::Path(p) => f.write_str(p.path.join("::").as_str()),
            FnArgType::Trait(p) => f.write_str(format!("dyn {}", p.path.join("::")).as_str()),
            FnArgType::Vec(p) => f.write_str(format!("Vec<{}>", p.path.join("::")).as_str()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FnArg {
    pub ty: FnArgType,
    pub ident: String,
    pub mutable: bool,
    pub ref_cnt: usize,
}

impl FnArg {
    pub fn new() -> Self {
        Self {
            ty: FnArgType::Path(Path::new()),
            ident: String::new(),
            mutable: false,
            ref_cnt: 0,
        }
    }

    pub fn parse_arg(&mut self, cr_idx: usize, super_path: &Vec<String>, arg: &syn::PatType) {
        if let syn::Pat::Ident(n) = &*arg.pat {
            self.ident = n.ident.to_string();
        }
        self.parse_type(cr_idx, super_path, &arg.ty);
    }

    fn parse_type(&mut self, cr_idx: usize, super_path: &Vec<String>, ty: &syn::Type) {
        match ty {
            syn::Type::Path(p) => {
                // Type with generic arguments
                if let Some(syn::PathArguments::AngleBracketed(ab)) =
                    p.path.segments.last().map(|s| &s.arguments)
                {
                    // Vec
                    if p.path.is_ident("Vec") {
                        self.ty = match ab.args.iter().collect::<Vec<_>>()[..] {
                            [syn::GenericArgument::Type(syn::Type::Path(p))] => {
                                FnArgType::Vec(Path {
                                    cr_idx,
                                    path: parse_syn_path(super_path, &p.path),
                                })
                            }
                            _ => {
                                panic!(
                                    "Expected single type path inside Vec<>, found {}",
                                    ab.args.to_token_stream().to_string()
                                )
                            }
                        }
                    }
                // Normal type
                } else {
                    self.ty = FnArgType::Path(Path {
                        cr_idx,
                        path: parse_syn_path(super_path, &p.path),
                    });
                }
            }
            syn::Type::Reference(r) => {
                self.ref_cnt += 1;
                self.mutable = r.mutability.is_some();
                self.parse_type(cr_idx, super_path, &r.elem);
            }
            syn::Type::TraitObject(t) => {
                for tpb in t.bounds.iter() {
                    match tpb {
                        syn::TypeParamBound::Trait(tr) => {
                            self.ty = FnArgType::Trait(Path {
                                cr_idx,
                                path: parse_syn_path(super_path, &tr.path),
                            });
                            break;
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
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
