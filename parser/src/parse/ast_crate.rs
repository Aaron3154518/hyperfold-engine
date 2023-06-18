use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::Read,
    iter::Map,
    path::PathBuf,
};

use shared::util::{Call, Catch, JoinMap, JoinMapInto};

use super::{
    ast_file::DirType,
    ast_mod::{AstMod, AstModType},
    AstSymbol,
};
use crate::{
    resolve::paths::{ExpandEnum, GetPaths, NamespaceTraits, Paths},
    util::end,
    validate::constants::NAMESPACE,
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

#[derive(Clone, Copy, Debug)]
enum AstCrateDependency {
    Crate(usize),
    MacrosCrate,
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

    pub fn parse(mut dir: PathBuf) -> (Vec<Self>, Paths) {
        let mut crates = vec![AstCrate::new(dir.to_owned(), 0, true)];

        let engine_dir = get_engine_dir();
        let macros_dir = get_macros_dir();
        let block_dirs = [(macros_dir.to_owned(), AstCrateDependency::MacrosCrate)]
            .into_iter()
            .collect::<HashMap<_, _>>();

        let mut crate_deps = Vec::new();
        let mut i = 0;
        while i < crates.len() {
            let cr_dir = crates[i].dir.to_owned();
            Self::get_crate_dependencies(cr_dir.to_owned(), &block_dirs, &crates).call_into(
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

        // Add macros crate at the end
        let macros_cr_idx = crates.len();
        crates.push(AstCrate::new(macros_dir.to_owned(), macros_cr_idx, false));
        crate_deps.push(Vec::new());

        // Insert correct dependencies
        for (cr, deps) in crates.iter_mut().zip(crate_deps.into_iter()) {
            cr.deps = deps
                .into_iter()
                .map(|(d, name)| {
                    (
                        match d {
                            AstCrateDependency::Crate(i) => i,
                            AstCrateDependency::MacrosCrate => macros_cr_idx,
                        },
                        name,
                    )
                })
                .collect()
        }

        // TODO: Auto detect game_crate macro (assume it exists and validate later)
        // Add namespace mod to all crates except macro
        // Must happen after the dependencies are set
        let end = end(&crates, 1);
        for cr in crates[..end].iter_mut() {
            cr.add_namespace_mod()
        }

        (crates, Paths::new(engine_cr_idx, macros_cr_idx))
    }

    fn get_crate_dependencies(
        cr_dir: PathBuf,
        block_dirs: &HashMap<PathBuf, AstCrateDependency>,
        crates: &Vec<AstCrate>,
    ) -> (Vec<(AstCrateDependency, String)>, Vec<String>) {
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
                    match block_dirs.get(&dep_dir) {
                        Some(d) => (*d, name),
                        None => match crates.iter().position(|cr| cr.dir == dep_dir) {
                            Some(i) => (AstCrateDependency::Crate(i), name),
                            None => {
                                new_deps.push(path);
                                (
                                    AstCrateDependency::Crate(crates.len() + new_deps.len() - 1),
                                    name,
                                )
                            }
                        },
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
    fn add_namespace_mod(&mut self) {
        let m = if self.idx == 0 {
            self.entry_namespace_mod()
        } else {
            self.dependency_namespace_mod()
        };
        self.main.mods.push(m);
    }

    fn entry_namespace_mod(&mut self) -> AstMod {
        let mut m = self.dependency_namespace_mod();
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

    fn dependency_namespace_mod(&mut self) -> AstMod {
        let mut path = self.main.path.to_vec();
        path.push(NAMESPACE.to_string());
        AstMod {
            ty: AstModType::Internal,
            dir: self.dir.to_owned(),
            path,
            mods: Vec::new(),
            // Traits
            symbols: NamespaceTraits::VARIANTS.iter().map_vec(|tr| AstSymbol {
                ident: tr.as_ident().to_string(),
                path: tr.full_path(),
                public: true,
            }),
            // Use dependency
            uses: self
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
}
