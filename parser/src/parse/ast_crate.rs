use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::Read,
    iter::Map,
    marker::PhantomData,
    path::PathBuf,
};

use diagnostic::{err, CatchErr, CombineWarnings, ErrForEach, ErrorTrait, ToErr};
use shared::{
    syn::error::{
        CriticalResult, GetVec, MutateResults, Result, SpanTrait, StrToError, WarningResult,
    },
    traits::{Call, Catch, CollectVec, CollectVecInto, ExpandEnum, GetSlice, PushInto},
};

use super::{
    ast_file::DirType,
    ast_mod::{AstMod, AstModType},
    AstItems, HardcodedSymbol, NewMod, Symbol, SymbolType,
};
use crate::{
    codegen::Crates,
    parse::ItemPath,
    utils::{
        constants::NAMESPACE,
        paths::Crate,
        tree::{FlattenTree, Tree},
    },
};

// TODO: hardcoded
const ENGINE: &str = ".";
const MACROS: &str = "macros";

fn get_engine_dir() -> CriticalResult<PathBuf> {
    fs::canonicalize(PathBuf::from(ENGINE)).catch_err(
        format!(
            "Could not canonicalize engine crate path relative to: {:#?}",
            env::current_dir()
        )
        .trace(),
    )
}

fn get_macros_dir() -> CriticalResult<PathBuf> {
    Ok(get_engine_dir()?.join("macros"))
}

#[derive(Debug)]
pub struct AstCrate {
    pub idx: usize,
    pub name: String,
    pub dir: PathBuf,
    pub main: usize,
    pub mods: Vec<AstMod>,
    pub deps: HashMap<usize, String>,
}

impl AstCrate {
    pub fn new(dir: PathBuf, idx: usize, is_entry: bool) -> Result<Self> {
        let rel_dir = dir.to_owned();
        let dir: PathBuf = fs::canonicalize(dir)
            .catch_err(format!("Could not canonicalize path: {}", rel_dir.display()).trace())?;

        let mut errs = Vec::new();
        let mods = AstMod::parse_dir(
            dir.join("src"),
            &vec!["crate".to_string()],
            if is_entry {
                DirType::Main
            } else {
                DirType::Lib
            },
        )?
        .flatten()
        .enumer_map_vec_into(|(i, (WarningResult { mut value, errors }, idxs))| {
            errs.extend(errors.into_iter().map(|e| e.with_mod(idx, i)));
            value.mods = idxs;
            value.idx = i;
            value
        });
        let main = mods.len() - 1;

        Ok(err(
            Self {
                idx,
                name: dir
                    .file_name()
                    .catch_err(format!("Could not parse file name: {}", rel_dir.display()).trace())?
                    .to_string_lossy()
                    .to_string(),
                dir: dir.to_owned(),
                main,
                mods,
                deps: HashMap::new(),
            },
            errs,
        ))
    }

    pub fn parse(mut dir: PathBuf) -> Result<Crates> {
        let engine_dir = get_engine_dir()?;
        let macros_dir = get_macros_dir()?;

        let mut crates = vec![AstCrate::new(dir.to_owned(), 0, true)?];
        let mut i = 0;
        while i < crates.len() {
            let cr_dir = crates.try_get(i)?.value.dir.to_owned();
            let (deps, new_deps) = Self::get_crate_dependencies(cr_dir.to_owned(), &crates)?;
            {
                crates.try_get_mut(i)?.value.deps = deps.into_iter().collect();
            }
            for path in new_deps {
                crates.push(AstCrate::new(cr_dir.join(path), crates.len(), false)?);
            }
            i += 1;
        }
        let crates = crates.combine_results();

        let mut crate_idxs = [0; Crate::LEN];
        crate_idxs[Crate::Main as usize] = 0;
        crate_idxs[Crate::Engine as usize] = crates
            .value
            .iter()
            .find_map(|cr| (cr.dir == engine_dir).then_some(cr.idx))
            .catch_err(
                format!("Could not find engine crate: '{}'", engine_dir.display()).trace(),
            )?;
        crate_idxs[Crate::Macros as usize] = crates
            .value
            .iter()
            .find_map(|cr| (cr.dir == macros_dir).then_some(cr.idx))
            .catch_err(
                format!("Could not find macros crate: '{}'", macros_dir.display()).trace(),
            )?;

        crates.try_map(|crates| Crates::new(crates, crate_idxs))
    }

    fn get_crate_dependencies(
        cr_dir: PathBuf,
        crates: &Vec<WarningResult<AstCrate>>,
    ) -> CriticalResult<(Vec<(usize, String)>, Vec<String>)> {
        let deps = AstCrate::parse_cargo_toml(cr_dir.to_owned())?;
        let mut new_deps = Vec::new();
        let mut cr_deps = Vec::new();
        for (name, path) in deps {
            let dep_dir = fs::canonicalize(cr_dir.join(path.to_string())).catch_err(
                format!(
                    "Could not canonicalize dependency path: {}: {}/{}",
                    name,
                    cr_dir.display(),
                    path
                )
                .trace(),
            )?;
            cr_deps.push(match crates.iter().position(|cr| cr.value.dir == dep_dir) {
                Some(i) => (i, name),
                None => {
                    new_deps.push(path);
                    (crates.len() + new_deps.len() - 1, name)
                }
            });
        }
        Ok((cr_deps, new_deps))
    }

    fn parse_cargo_toml(dir: PathBuf) -> CriticalResult<HashMap<String, String>> {
        // Get the path to the `Cargo.toml` file
        let cargo_toml_path = dir.join("Cargo.toml");

        // Read the `Cargo.toml` file into a string
        let mut cargo_toml = String::new();
        let mut file = File::open(&cargo_toml_path).catch_err(
            format!("Could not open Cargo.toml: {}", cargo_toml_path.display()).trace(),
        )?;
        file.read_to_string(&mut cargo_toml).catch_err(
            format!("Could not read Cargo.toml: {}", cargo_toml_path.display()).trace(),
        )?;

        // Parse the `Cargo.toml` file as TOML
        let cargo_toml = cargo_toml.parse::<toml::Value>().catch_err(
            format!("Could not parse Cargo.toml: {}", cargo_toml_path.display()).trace(),
        )?;

        // Extract the list of dependencies from the `Cargo.toml` file
        let deps = cargo_toml
            .get("dependencies")
            .catch_err("Could not find 'dependencies' section in Cargo.toml".trace())?
            .as_table()
            .catch_err("Could not convert 'dependencies' section to a table".trace())?;
        Ok(deps
            .into_iter()
            .filter_map(|(k, v)| match v {
                toml::Value::Table(t) => match (t.get("path"), t.get("dependency")) {
                    (Some(toml::Value::String(p)), Some(_)) => Some((k.to_string(), p.to_string())),
                    _ => None,
                },
                _ => None,
            })
            .collect())
    }

    // Insert things into crates
    pub fn add_symbol(&mut self, sym: Symbol) -> CriticalResult<()> {
        self.find_mod_mut(sym.path.slice_to(-1))?.symbols.push(sym);
        Ok(())
    }

    pub fn add_hardcoded_symbol(crates: &mut Crates, sym: HardcodedSymbol) -> CriticalResult<()> {
        let path = sym.get_path();
        crates.get_crate_mut(path.cr)?.add_symbol(Symbol {
            kind: SymbolType::Hardcoded(sym),
            path: path.full_path(),
            public: true,
        });
        Ok(())
    }
}

// Mod access
impl AstCrate {
    pub fn get_main_mod(&self) -> CriticalResult<&AstMod> {
        self.get_mod(self.main)
    }

    pub fn get_main_mod_mut(&mut self) -> CriticalResult<&mut AstMod> {
        self.get_mod_mut(self.main)
    }

    pub fn get_mod(&self, i: usize) -> CriticalResult<&AstMod> {
        self.mods.try_get(i)
    }

    pub fn get_mod_mut(&mut self, i: usize) -> CriticalResult<&mut AstMod> {
        self.mods.try_get_mut(i)
    }

    pub fn get_mods(&self, idxs: &Vec<usize>) -> CriticalResult<Vec<&AstMod>> {
        idxs.try_for_each(|i| self.get_mod(*i)).critical()
    }

    pub fn find_mod<'a>(&'a self, path: &[String]) -> CriticalResult<&'a AstMod> {
        self.iter_mods().find(|m| m.path == path).ok_or(
            format!("No mod defined at path: {path:#?}")
                .trace()
                .as_vec(),
        )
    }

    pub fn find_mod_mut<'a>(&'a mut self, path: &[String]) -> CriticalResult<&'a mut AstMod> {
        self.iter_mods_mut().find(|m| m.path == path).ok_or(
            format!("No mod defined at path: {path:#?}")
                .trace()
                .as_vec(),
        )
    }

    pub fn iter_mods(&self) -> impl Iterator<Item = &AstMod> {
        self.mods.iter()
    }

    pub fn iter_mods_mut(&mut self) -> impl Iterator<Item = &mut AstMod> {
        self.mods.iter_mut()
    }
}
