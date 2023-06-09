use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::Read,
    iter::Map,
    marker::PhantomData,
    path::PathBuf,
};

use shared::util::{Call, Catch, JoinMap, JoinMapInto, PushInto};

use super::{
    ast_file::DirType,
    ast_mod::{AstMod, AstModType, SymbolType},
    HardcodedSymbol, Symbol,
};
use crate::{
    codegen2::Crates,
    resolve::constants::NAMESPACE,
    resolve::{Crate, ExpandEnum, ItemPath},
    util::end,
};

// TODO: hardcoded
const ENGINE: &str = ".";
const MACROS: &str = "macros";

fn get_engine_dir() -> PathBuf {
    fs::canonicalize(PathBuf::from(ENGINE)).catch(format!(
        "Could not canonicalize engine crate path relative to: {:#?}",
        env::current_dir()
    ))
}

fn get_macros_dir() -> PathBuf {
    get_engine_dir().join("macros")
}

#[derive(Debug)]
pub struct AstCrate {
    pub idx: usize,
    pub name: String,
    pub dir: PathBuf,
    pub main: AstMod,
    pub deps: HashMap<usize, String>,
}

impl AstCrate {
    pub fn new(dir: PathBuf, idx: usize, is_entry: bool) -> Self {
        let rel_dir = dir.to_owned();
        let dir: PathBuf = fs::canonicalize(dir).catch(format!(
            "Could not canonicalize path: {}",
            rel_dir.display()
        ));

        Self {
            idx,
            name: dir
                .file_name()
                .catch(format!("Could not parse file name: {}", rel_dir.display()))
                .to_string_lossy()
                .to_string(),
            dir: dir.to_owned(),
            main: AstMod::parse_dir(
                dir.join("src"),
                &vec!["crate".to_string()],
                if is_entry {
                    DirType::Main
                } else {
                    DirType::Lib
                },
            ),
            deps: HashMap::new(),
        }
    }

    pub fn parse(mut dir: PathBuf) -> Crates {
        let mut crates = vec![AstCrate::new(dir.to_owned(), 0, true)];

        let engine_dir = get_engine_dir();
        let macros_dir = get_macros_dir();

        let mut crate_deps = Vec::new();
        let mut i = 0;
        while i < crates.len() {
            let cr_dir = crates[i].dir.to_owned();
            Self::get_crate_dependencies(cr_dir.to_owned(), &crates).call_into(
                |(deps, new_deps)| {
                    crate_deps.push(deps);
                    for path in new_deps {
                        crates.push(AstCrate::new(cr_dir.join(path), crates.len(), false))
                    }
                },
            );
            i += 1;
        }

        let engine_cr_idx = crates
            .iter()
            .find_map(|cr| (cr.dir == engine_dir).then_some(cr.idx))
            .expect("Could not find engine crate");
        let macros_cr_idx = crates
            .iter()
            .find_map(|cr| (cr.dir == macros_dir).then_some(cr.idx))
            .expect("Could not find macros crate");

        // Insert correct dependencies
        for (cr, deps) in crates.iter_mut().zip(crate_deps.into_iter()) {
            cr.deps = deps.into_iter().collect();
        }

        let mut crate_idxs = [0; Crate::LEN];
        crate_idxs[Crate::Main as usize] = 0;
        crate_idxs[Crate::Engine as usize] = engine_cr_idx;
        crate_idxs[Crate::Macros as usize] = macros_cr_idx;

        Crates::new(crates, crate_idxs)
    }

    fn get_crate_dependencies(
        cr_dir: PathBuf,
        crates: &Vec<AstCrate>,
    ) -> (Vec<(usize, String)>, Vec<String>) {
        let deps = AstCrate::parse_cargo_toml(cr_dir.to_owned());
        let mut new_deps = Vec::new();
        (
            deps.into_iter()
                .map(|(name, path)| {
                    let dep_dir = fs::canonicalize(cr_dir.join(path.to_string())).catch(format!(
                        "Could not canonicalize dependency path: {}: {}/{}",
                        name,
                        cr_dir.display(),
                        path
                    ));
                    match crates.iter().position(|cr| cr.dir == dep_dir) {
                        Some(i) => (i, name),
                        None => {
                            new_deps.push(path);
                            (crates.len() + new_deps.len() - 1, name)
                        }
                    }
                })
                .collect(),
            new_deps,
        )
    }

    fn parse_cargo_toml(dir: PathBuf) -> HashMap<String, String> {
        // Get the path to the `Cargo.toml` file
        let cargo_toml_path = dir.join("Cargo.toml");

        // Read the `Cargo.toml` file into a string
        let mut cargo_toml = String::new();
        let mut file = File::open(&cargo_toml_path).catch(format!(
            "Could not open Cargo.toml: {}",
            cargo_toml_path.display()
        ));
        file.read_to_string(&mut cargo_toml).catch(format!(
            "Could not read Cargo.toml: {}",
            cargo_toml_path.display()
        ));

        // Parse the `Cargo.toml` file as TOML
        let cargo_toml = cargo_toml.parse::<toml::Value>().catch(format!(
            "Could not parse Cargo.toml: {}",
            cargo_toml_path.display()
        ));

        // Extract the list of dependencies from the `Cargo.toml` file
        let deps = cargo_toml
            .get("dependencies")
            .expect("Could not find 'dependencies' section in Cargo.toml")
            .as_table()
            .expect("Could not convert 'dependencies' section to a table");
        deps.into_iter()
            .filter_map(|(k, v)| match v {
                toml::Value::Table(t) => match (t.get("path"), t.get("dependency")) {
                    (Some(toml::Value::String(p)), Some(_)) => Some((k.to_string(), p.to_string())),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    // Adds AstMod for the namespace module generated in codegen
    // fn add_namespace_mod(&mut self) {
    //     if self.idx == 0 {
    //         self.gen_entry_namespace_mod();
    //     } else {
    //         self.gen_dependency_namespace_mod();
    //     }
    // }

    // fn gen_entry_namespace_mod(&mut self) -> &mut AstMod {
    //     let mut m = self.gen_dependency_namespace_mod();
    //     // Foo structs
    //     for tr in NamespaceTraits::VARIANTS.iter() {
    //         let gl = tr.get_global();
    //         let sym = AstSymbol {
    //             kind: AstSymbolType::Hardcoded,
    //             ident: gl.as_ident().to_string(),
    //             path: gl.full_path(),
    //             public: true,
    //         };
    //         m.symbols.push(sym.to_owned());
    //     }
    //     m
    // }

    // fn gen_dependency_namespace_mod(&mut self) -> &mut AstMod {
    //     let mut m = self.add_mod(self.main.path.to_vec().push_into(NAMESPACE.to_string()));
    //     // Traits
    //     // for tr in NamespaceTraits::VARIANTS {
    //     //     m.symbols.push(AstSymbol {
    //     //         kind: AstSym,
    //     //         path: (),
    //     //         public: (),
    //     //     })
    //     // }
    //     // m.symbols = NamespaceTraits::VARIANTS.iter().map_vec(|tr| AstSymbol {
    //     //     kind: AstSymbolType::Hardcoded,
    //     //     ident: tr.as_ident().to_string(),
    //     //     path: tr.full_path(),
    //     //     public: true,
    //     // });
    //     // Use dependency
    //     // m.uses = self
    //     //     .deps
    //     //     .iter()
    //     //     .map(|(_, alias)| AstSymbol {
    //     //         kind: AstSymbolType::Hardcoded,
    //     //         ident: alias.to_string(),
    //     //         path: vec!["crate", &alias].map_vec(|s| s.to_string()),
    //     //         public: true,
    //     //     })
    //     //     .collect();
    //     m
    // }

    // Insert things into crates
    fn get_mod_from_path<'a>(&'a mut self, path: &[String]) -> &'a mut AstMod {
        let mut path_it = path.iter();
        if !path_it.next().is_some_and(|s| s == "crate") {
            panic!("No mod defined at the path: {path:#?}")
        }
        let mut m = &mut self.main;
        while let Some(ident) = path_it.next() {
            m = m
                .mods
                .iter_mut()
                .find(|m| m.path.last().is_some_and(|p| p == ident))
                .catch(format!("No mod defined at the path: {path:#?}"));
        }
        m
    }

    pub fn add_symbol(&mut self, sym: Symbol) {
        self.get_mod_from_path(&sym.path[..end(&sym.path, 1)])
            .symbols
            .push(sym);
    }

    pub fn add_mod<'a>(&'a mut self, path: Vec<String>) -> &'a mut AstMod {
        let m = self.get_mod_from_path(&path[..end(&path, 1)]);
        m.mods.push(AstMod::new(
            m.dir.to_owned(),
            path.to_vec(),
            AstModType::Internal,
        ));
        m.mods.last_mut().catch(format!("Mod not added: {path:#?}"))
    }

    pub fn add_hardcoded_symbol(crates: &mut Crates, sym: HardcodedSymbol) {
        let path = sym.get_path();
        crates.get_crate_mut(path.cr).add_symbol(Symbol {
            kind: SymbolType::Hardcoded(sym),
            path: path.full_path(),
            public: true,
        });
    }

    pub fn iter_mods_mut(&self) -> MutIter {
        MutIter::new(self)
    }

    pub fn iter_mods<'a>(&'a self) -> impl Iterator<Item = &'a AstMod> {
        self.get_mods().into_iter()
    }

    pub fn get_mods<'a>(&'a self) -> Vec<&'a AstMod> {
        Self::add_mods(&self.main, Vec::new())
    }

    fn add_mods<'a>(m: &'a AstMod, mut mods: Vec<&'a AstMod>) -> Vec<&'a AstMod> {
        mods.push(m);
        for m in m.mods.iter() {
            mods = Self::add_mods(m, mods)
        }
        mods
    }
}

pub struct MutIter {
    idxs: <Vec<Vec<usize>> as IntoIterator>::IntoIter,
}

impl MutIter {
    pub fn new(cr: &AstCrate) -> Self {
        Self {
            idxs: Self::get_idxs(&cr.main, Vec::new(), Vec::new()).into_iter(),
        }
    }

    fn get_idxs(m: &AstMod, curr_idxs: Vec<usize>, mut idxs: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
        idxs.push(curr_idxs.to_vec());
        for (i, m) in m.mods.iter().enumerate() {
            idxs = Self::get_idxs(m, curr_idxs.to_vec().push_into(i), idxs)
        }
        idxs
    }

    pub fn next<'a>(&mut self, cr: &'a mut AstCrate) -> Option<&'a mut AstMod> {
        match self.idxs.next() {
            Some(idxs) => {
                let mut m = &mut cr.main;
                for i in idxs {
                    m = match m.mods.get_mut(i) {
                        Some(m) => m,
                        None => return None,
                    }
                }
                Some(m)
            }
            None => None,
        }
    }
}
