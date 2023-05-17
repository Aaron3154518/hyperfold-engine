use std::path::PathBuf;

use shared::parse_args::GlobalMacroArgs;

use crate::{
    parse::{
        ast_crate::Crate,
        ast_mod::{MarkType, MarkedItem, Mod, ModType, Symbol},
    },
    resolve::{
        ast_items::{Global, ItemsCrate},
        ast_paths::{
            EngineGlobals, EngineTraits, ExpandEnum, GetPaths, MacroPaths, NamespaceTraits,
        },
        ast_resolve::Path,
    },
    validate::constants::NAMESPACE,
};

use shared::util::{JoinMap, JoinMapInto};

use super::idents::Idents;

pub fn entry_namespace_items(items: &mut ItemsCrate) {
    for tr in NamespaceTraits::VARIANTS.iter() {
        items.globals.push(Global {
            path: Path {
                cr_idx: items.cr_idx,
                path: tr.get_global().full_path(),
            },
            args: GlobalMacroArgs::from(Vec::new()),
        });
    }
}

pub fn entry_namespace_mod(cr: &Crate, dir: PathBuf, mods: Vec<String>) -> Mod {
    let mut m = dependency_namespace_mod(cr, dir, mods);
    // Foo structs
    for tr in NamespaceTraits::VARIANTS.iter() {
        let gl = tr.get_global();
        let sym = Symbol {
            ident: gl.as_ident().to_string(),
            path: gl.full_path(),
            public: true,
        };
        m.symbols.push(sym.to_owned());
    }
    m
}

pub fn dependency_namespace_mod(cr: &Crate, dir: PathBuf, mut mods: Vec<String>) -> Mod {
    mods.push(NAMESPACE.to_string());
    Mod {
        ty: ModType::Internal,
        dir: dir.to_owned(),
        path: mods,
        mods: Vec::new(),
        // Traits
        symbols: NamespaceTraits::VARIANTS.iter().map_vec(|tr| Symbol {
            ident: tr.as_ident().to_string(),
            path: tr.full_path(),
            public: true,
        }),
        // Use dependency
        uses: cr
            .deps
            .iter()
            .map(|(_, alias)| Symbol {
                ident: alias.to_string(),
                path: vec!["crate", &alias].map_vec(|s| s.to_string()),
                public: true,
            })
            .collect(),
        marked: Vec::new(),
    }
}
