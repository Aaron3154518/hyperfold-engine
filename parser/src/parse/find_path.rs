use syn::Pat;

use crate::{
    parse::{
        AstCrate, ModInfo, {AstMod, Symbol},
    },
    utils::{syn::use_path_from_syn, Msg, MsgResult},
};
use shared::traits::Catch;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ItemPath {
    pub cr_idx: usize,
    pub path: Vec<String>,
}

impl ItemPath {
    pub fn new() -> Self {
        Self {
            cr_idx: 0,
            path: Vec::new(),
        }
    }
}

pub type ResolveResult<'a> = Result<&'a Symbol, ItemPath>;

// Err means:
// 1) Not from a valid crate
// 2) resolve_mod() returns Err
pub fn resolve_path_from_crate<'a>(
    mut path: Vec<String>,
    cr: &'a AstCrate,
    crates: &'a Vec<AstCrate>,
) -> ResolveResult<'a> {
    // println!("Resolve: {}, crate: {}", path.join("::"), cr.idx);
    let cr_idx = cr.idx;
    match path.first() {
        Some(p) => {
            match p.as_str() {
                // Match this crate
                "crate" => Some(resolve_path_from_mod(
                    path.to_vec(),
                    1,
                    (&cr.main, cr, crates),
                )),
                // Match dependency
                _ => cr.deps.iter().find_map(|(idx, alias)| {
                    (alias == p).then(|| {
                        let cr = crates.get(*idx).expect("Invalid dependency index");
                        resolve_path_from_crate(
                            [vec!["crate".to_string()], path[1..].to_vec()].concat(),
                            cr,
                            crates,
                        )
                    })
                }),
            }
            .map_or(Err(ItemPath { cr_idx, path }), |r| r)
        }
        None => Err(ItemPath { cr_idx, path }),
    }
}

// Assumes we already match with mod
// Err if:
// 1) Does not match anything
// 2) Matches mod but remainder doesn't match
// 3) Matches use statement new path doesn't match
fn resolve_path_from_mod<'a>(
    path: Vec<String>,
    idx: usize,
    (m, cr, crates): ModInfo<'a>,
) -> ResolveResult<'a> {
    // println!(
    //     "Resolve Mod: {} at {}",
    //     path.join("::"),
    //     path.get(idx).unwrap_or(&"None".to_string())
    // );
    let cr_idx = cr.idx;

    let name = path
        .get(idx)
        .catch(format!(
            "Bad resolve path index: {} in path: \"{}\"",
            idx,
            path.join("::")
        ))
        .to_string();
    let is_path_end = idx == path.len() - 1;

    // println!("Finding: {name}");
    // Check sub modules
    for m in m.mods.iter() {
        if name == *m.path.last().expect("Mod path is empty") {
            // println!("Found Mod: {}", name);
            return if is_path_end {
                // The path points to a mod
                Err(ItemPath { cr_idx, path })
            } else {
                resolve_path_from_mod(path, idx + 1, (m, cr, crates))
            };
        }
    }
    // Check symbols
    // TODO: previously filtered public
    for sym in m.symbols.iter() {
        if sym.path.last().is_some_and(|s| s == &name) {
            // println!("Found Symbol: {}", sym.path.join("::"));
            return if is_path_end {
                Ok(sym)
            } else {
                Err(ItemPath {
                    cr_idx: cr.idx,
                    path: sym.path.to_vec(),
                })
            };
        }
    }
    // Check use statements
    for sym in m.uses.iter() {
        // Glob - this is allowed to fail
        if sym.ident == "*" {
            let path = [sym.path.to_vec(), path[idx..].to_vec()].concat();
            if let Ok(v) = resolve_path(path.to_vec(), (m, cr, crates)) {
                return Ok(v);
            }
        // Use
        } else if sym.ident == name {
            let path = [sym.path.to_vec(), path[idx + 1..].to_vec()].concat();
            // println!("Matched Use: {}", sym.path.join("::"));
            return resolve_path(path.to_vec(), (m, cr, crates));
        }
    }

    Err(ItemPath { cr_idx, path })
}

// Paths that start relative to some mod item
pub fn resolve_path<'a>(path: Vec<String>, (m, cr, crates): ModInfo<'a>) -> ResolveResult<'a> {
    // println!("Local Resolve: {}", path.join("::"));
    let cr_idx = cr.idx;

    let name = path
        .first()
        .catch(format!("Empty resolve path: {}", path.join("::")));

    // Can't be local
    if name == "crate" {
        return resolve_path_from_crate(path, cr, crates);
    }

    // Search for a symbol
    if path.len() == 1 {
        if let Some(sym) = m
            .symbols
            .iter()
            .find_map(|sym| sym.path.ends_with(&[name.to_string()]).then_some(sym))
        {
            return Ok(sym);
        }
    }

    // Check mods
    if let Some(m) = m
        .mods
        .iter()
        .find(|m| m.path.last().is_some_and(|s| s == name))
    {
        return resolve_path_from_crate([m.path.to_vec(), path[1..].to_vec()].concat(), cr, crates);
    }

    // Check possible paths
    if let Some(res) = m.uses.iter().find_map(|syn| {
        // Get possible path
        if syn.ident == "*" {
            resolve_path_from_crate([syn.path.to_vec(), path.to_vec()].concat(), cr, crates)
                .map_or(None, |p| Some(Ok(p)))
        } else if name == &syn.ident {
            Some(resolve_path_from_crate(
                [syn.path.to_vec(), path[1..].to_vec()].concat(),
                cr,
                crates,
            ))
        } else {
            None
        }
    }) {
        return res;
    }

    // Check dependencies
    resolve_path_from_crate(path, cr, crates)
}

pub fn resolve_syn_path<'a>(
    parent_path: &Vec<String>,
    path: &syn::Path,
    (m, cr, crates): ModInfo<'a>,
) -> ResolveResult<'a> {
    resolve_path(use_path_from_syn(&m.path, path), (m, cr, crates))
}

pub trait ResolveResultTrait<'a> {
    fn expect_symbol(self) -> MsgResult<&'a Symbol>;
}

impl<'a> ResolveResultTrait<'a> for ResolveResult<'a> {
    fn expect_symbol(self) -> MsgResult<&'a Symbol> {
        match self {
            Ok(sym) => Ok(sym),
            Err(e) => Err(vec![Msg::String(format!(
                "Could not resolve path: {}",
                e.path.join("::")
            ))]),
        }
    }
}
