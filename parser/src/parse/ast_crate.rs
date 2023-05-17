use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::Read,
    iter::Map,
    path::PathBuf,
};

use shared::util::{Call, Catch};

use super::{
    ast_file::DirType,
    ast_mod::{Mod, ModType},
};
use crate::{
    codegen::mods::{dependency_namespace_mod, entry_namespace_mod},
    resolve::ast_paths::Paths,
    util::end,
};

// TODO: hardcoded
const ENGINE: &str = ".";
const MACROS: &str = "macros";

pub fn get_engine_dir() -> PathBuf {
    fs::canonicalize(PathBuf::from(ENGINE)).catch(format!(
        "Could not canonicalize engine crate path relative to: {:#?}",
        env::current_dir()
    ))
}

pub fn get_macros_dir() -> PathBuf {
    get_engine_dir().join("macros")
}

#[derive(Clone, Copy, Debug)]
enum Dependency {
    Crate(usize),
    MacrosCrate,
}

#[derive(Debug)]
pub struct Crate {
    pub idx: usize,
    pub name: String,
    pub dir: PathBuf,
    pub main: Mod,
    pub deps: HashMap<usize, String>,
}

impl Crate {
    pub fn new(dir: PathBuf, idx: usize, is_entry: bool) -> Self {
        let err_dir = dir.to_owned();
        let dir: PathBuf = fs::canonicalize(dir).catch(format!(
            "Could not canonicalize path: {}",
            err_dir.display()
        ));

        Self {
            idx,
            name: dir
                .file_name()
                .catch(format!("Could not parse file name: {}", err_dir.display()))
                .to_string_lossy()
                .to_string(),
            dir: dir.to_owned(),
            main: Mod::parse_dir(
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

    fn get_crate_dependencies(
        cr_dir: PathBuf,
        block_dirs: &HashMap<PathBuf, Dependency>,
        crates: &Vec<Crate>,
    ) -> (Vec<(Dependency, String)>, Vec<String>) {
        let deps = Crate::parse_cargo_toml(cr_dir.to_owned());
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
                            Some(i) => (Dependency::Crate(i), name),
                            None => {
                                new_deps.push(path);
                                (Dependency::Crate(crates.len() + new_deps.len() - 1), name)
                            }
                        },
                    }
                })
                .collect(),
            new_deps,
        )
    }

    pub fn parse(mut dir: PathBuf) -> (Vec<Self>, Paths) {
        let mut crates = vec![Crate::new(dir.to_owned(), 0, true)];

        let engine_dir = get_engine_dir();
        let macros_dir = get_macros_dir();
        let block_dirs = [(macros_dir.to_owned(), Dependency::MacrosCrate)]
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
                        crates.push(Crate::new(cr_dir.join(path), crates.len(), false))
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
        crates.push(Crate::new(macros_dir.to_owned(), macros_cr_idx, false));
        crate_deps.push(Vec::new());

        // Insert correct dependencies
        for (cr, deps) in crates.iter_mut().zip(crate_deps.into_iter()) {
            cr.deps = deps
                .into_iter()
                .map(|(d, name)| {
                    (
                        match d {
                            Dependency::Crate(i) => i,
                            Dependency::MacrosCrate => macros_cr_idx,
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
        for (i, cr) in crates[..end].iter_mut().enumerate() {
            cr.main.mods.push(if i == 0 {
                entry_namespace_mod(&cr, cr.main.dir.to_owned(), cr.main.path.to_vec())
            } else {
                dependency_namespace_mod(&cr, cr.main.dir.to_owned(), cr.main.path.to_vec())
            });
        }

        (crates, Paths::new(engine_cr_idx, macros_cr_idx))
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
}
