use std::path::PathBuf;

use shared::parse_args::GlobalMacroArgs;

use crate::{
    parse::{
        ast_crate::AstCrate,
        ast_mod::{AstItemType, AstMod, AstModType, AstSymbol, MarkedAstItem},
    },
    resolve::{
        items_crate::{ItemGlobal, ItemTrait, ItemsCrate},
        path::ItemPath,
        paths::{EngineGlobals, EngineTraits, ExpandEnum, GetPaths, MacroPaths, NamespaceTraits},
    },
    validate::constants::NAMESPACE,
};

use shared::util::{JoinMap, JoinMapInto};

use super::idents::Idents;

// Add namespace traits and globals, happens at resolve-time
pub fn add_traits(items: &mut Vec<ItemsCrate>) {
    let entry = items
        .iter_mut()
        .find(|cr| cr.cr_idx == 0)
        .expect("No entry crate");

    let traits = NamespaceTraits::VARIANTS.map(|tr| {
        // Add trait
        let new_tr = ItemTrait {
            path: ItemPath {
                cr_idx: entry.cr_idx,
                path: tr.full_path(),
            },
            g_idx: entry.globals.len(),
        };
        // Add trait global
        entry.globals.push(ItemGlobal {
            path: ItemPath {
                cr_idx: entry.cr_idx,
                path: tr.get_global().full_path(),
            },
            args: GlobalMacroArgs::from(Vec::new()),
        });
        new_tr
    });

    drop(entry);
    for i in items.iter_mut() {
        traits.iter().for_each(|tr| i.traits.push(tr.clone()));
    }
}

// Add namespace mod, happends as parse-time
pub fn entry_namespace_mod(cr: &AstCrate, dir: PathBuf, mods: Vec<String>) -> AstMod {
    let mut m = dependency_namespace_mod(cr, dir, mods);
    // Foo structs
    for tr in NamespaceTraits::VARIANTS.iter() {
        let gl = tr.get_global();
        let sym = AstSymbol {
            ident: gl.as_ident().to_string(),
            path: gl.full_path(),
            public: true,
        };
        m.symbols.push(sym.to_owned());
    }
    m
}

pub fn dependency_namespace_mod(cr: &AstCrate, dir: PathBuf, mut mods: Vec<String>) -> AstMod {
    mods.push(NAMESPACE.to_string());
    AstMod {
        ty: AstModType::Internal,
        dir: dir.to_owned(),
        path: mods,
        mods: Vec::new(),
        // Traits
        symbols: NamespaceTraits::VARIANTS.iter().map_vec(|tr| AstSymbol {
            ident: tr.as_ident().to_string(),
            path: tr.full_path(),
            public: true,
        }),
        // Use dependency
        uses: cr
            .deps
            .iter()
            .map(|(_, alias)| AstSymbol {
                ident: alias.to_string(),
                path: vec!["crate", &alias].map_vec(|s| s.to_string()),
                public: true,
            })
            .collect(),
        marked: Vec::new(),
        macro_calls: Vec::new(),
    }
}
