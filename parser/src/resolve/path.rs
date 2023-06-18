use syn::Pat;

use crate::parse::{
    AstCrate, {AstMod, AstSymbol},
};
use shared::util::Catch;

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

// Err means:
// 1) Not from a valid crate
// 2) resolve_mod() returns Err
fn resolve_path_from_crate(
    mut path: Vec<String>,
    cr: &AstCrate,
    crates: &Vec<AstCrate>,
) -> Result<ItemPath, ItemPath> {
    // println!("Resolve: {}, crate: {}", path.join("::"), cr.idx);
    let cr_idx = cr.idx;
    match path.first() {
        Some(p) => {
            match p.as_str() {
                // Match this crate
                "crate" => Some(resolve_path_from_mod(
                    path.to_vec(),
                    1,
                    cr,
                    &cr.main,
                    crates,
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
fn resolve_path_from_mod(
    path: Vec<String>,
    idx: usize,
    cr: &AstCrate,
    m: &AstMod,
    crates: &Vec<AstCrate>,
) -> Result<ItemPath, ItemPath> {
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
    // println!("Finding: {name}");
    // Check sub modules
    for m in m.mods.iter() {
        if name == *m.path.last().expect("Mod path is empty") {
            // println!("Found Mod: {}", name);
            return if idx + 1 == path.len() {
                // The path points to a mod
                Ok(ItemPath { cr_idx, path })
            } else {
                return resolve_path_from_mod(path, idx + 1, cr, m, crates);
            };
        }
    }
    // Check symbols
    // TODO: previously filtered public
    for sym in m.symbols.iter() {
        if sym.ident == name {
            // println!("Found Symbol: {}", sym.path.join("::"));
            return Ok(ItemPath {
                cr_idx: cr.idx,
                path: sym.path.to_vec(),
            });
        }
    }
    // Check use statements
    for sym in m.uses.iter() {
        // Glob - this is allowed to fail
        if sym.ident == "*" {
            let path = [sym.path.to_vec(), path[idx..].to_vec()].concat();
            if let Ok(v) = resolve_path(path.to_vec(), cr, m, crates) {
                return Ok(v);
            }
        // Use
        } else if sym.ident == name {
            let path = [sym.path.to_vec(), path[idx + 1..].to_vec()].concat();
            // println!("Matched Use: {}", sym.path.join("::"));
            return resolve_path(path.to_vec(), cr, m, crates);
        }
    }
    Err(ItemPath { cr_idx, path })
}

// Paths that start relative to some mod item
pub fn resolve_path(
    path: Vec<String>,
    cr: &AstCrate,
    m: &AstMod,
    crates: &Vec<AstCrate>,
) -> Result<ItemPath, ItemPath> {
    // println!("Local Resolve: {}", path.join("::"));
    let cr_idx = cr.idx;

    let name = path
        .first()
        .catch(format!("Empty resolve path: {}", path.join("::")));

    // Can't be local
    if name == "crate" {
        return resolve_path_from_crate(path, cr, crates);
    }

    // Iterate possible paths
    [&m.symbols, &m.uses]
        .iter()
        .find_map(|syns| {
            syns.iter().find_map(|syn| {
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
            })
        })
        .unwrap_or_else(|| resolve_path_from_crate(path, cr, crates))
}
