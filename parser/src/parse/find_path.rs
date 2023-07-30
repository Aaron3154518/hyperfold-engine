use quote::ToTokens;
use syn::{spanned::Spanned, Pat};

use crate::parse::{
    AstCrate, ModInfo, {AstMod, Symbol},
};
use shared::{
    syn::{use_path_from_syn, CatchErr, InjectSpan, Msg, MsgResult},
    traits::{Catch, CollectVecInto},
};

use super::symbol::MatchSymbolTrait;

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

fn resolve_error(path: Vec<String>) -> Vec<Msg> {
    let path = path.join("::");
    vec![Msg::String(format!("Could not resolve path: {path}"))]
}

// Err means:
// 1) Not from a valid crate
// 2) resolve_mod() returns Err
pub fn resolve_path_from_crate<'a>(
    mut path: Vec<String>,
    cr: &'a AstCrate,
    crates: &'a Vec<AstCrate>,
) -> MsgResult<&'a Symbol> {
    // println!("Resolve: {}, crate: {}", path.join("::"), cr.idx);
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
                        resolve_path_from_crate(
                            [vec!["crate".to_string()], path[1..].to_vec()].concat(),
                            crates.get(*idx).catch_err("Invalid dependency index")?,
                            crates,
                        )
                    })
                }),
            }
            .map_or(Err(resolve_error(path)), |r| r)
        }
        None => Err(resolve_error(path)),
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
) -> MsgResult<&'a Symbol> {
    // println!(
    //     "Resolve Mod: {} at {}",
    //     path.join("::"),
    //     path.get(idx).unwrap_or(&"None".to_string())
    // );
    let cr_idx = cr.idx;

    let name = path
        .get(idx)
        .catch_err(&format!(
            "Bad resolve path index: {} in path: \"{}\"",
            idx,
            path.join("::")
        ))?
        .to_string();
    let is_path_end = idx == path.len() - 1;

    // println!("Finding: {name}");
    // Check sub modules
    for m in m.mods.iter() {
        if name == *m.path.last().catch_err("Mod path is empty")? {
            // println!("Found Mod: {}", name);
            return if is_path_end {
                // The path points to a mod
                Err(resolve_error(path))
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
                Err(resolve_error(sym.path.to_vec()))
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

    Err(resolve_error(path))
}

// Paths that start relative to some mod item
pub fn resolve_path<'a>(path: Vec<String>, (m, cr, crates): ModInfo<'a>) -> MsgResult<&'a Symbol> {
    // println!("Local Resolve: {}", path.join("::"));
    let cr_idx = cr.idx;

    let name = path
        .first()
        .catch_err(&format!("Empty resolve path: {}", path.join("::")))?;

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
) -> MsgResult<&'a Symbol> {
    resolve_path(use_path_from_syn(&m.path, path), (m, cr, crates))
}

impl<'a> MatchSymbolTrait<'a> for MsgResult<&'a Symbol> {
    fn and_then_symbol<T>(self, f: impl FnOnce(&'a Symbol) -> MsgResult<T>) -> MsgResult<T> {
        self.and_then(f)
    }

    fn and_then_symbol_in_mod<T>(
        self,
        f: impl FnOnce(&'a Symbol) -> MsgResult<T>,
        m: &AstMod,
        span: &dyn Spanned,
    ) -> MsgResult<T> {
        self.in_mod(m, span).and_then(f)
    }
}
