use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::Read,
    iter::Map,
    marker::PhantomData,
    path::PathBuf,
};

use shared::traits::{Call, Catch, CollectVec, CollectVecInto, ExpandEnum, GetSlice, PushInto};

use super::{
    ast_file::DirType,
    ast_mod::{AstMod, AstModType},
    HardcodedSymbol, Symbol, SymbolType,
};
use crate::{
    codegen::Crates,
    parse::ItemPath,
    utils::{constants::NAMESPACE, paths::Crate, CatchErr, GetVec, Msg, MsgResult, SpanFiles},
};

// TODO: hardcoded
const ENGINE: &str = ".";
const MACROS: &str = "macros";

fn get_engine_dir() -> MsgResult<PathBuf> {
    fs::canonicalize(PathBuf::from(ENGINE)).catch_err(&format!(
        "Could not canonicalize engine crate path relative to: {:#?}",
        env::current_dir()
    ))
}

fn get_macros_dir() -> MsgResult<PathBuf> {
    Ok(get_engine_dir()?.join("macros"))
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
    pub fn new(
        span_files: &mut SpanFiles,
        dir: PathBuf,
        idx: usize,
        is_entry: bool,
    ) -> MsgResult<Self> {
        let rel_dir = dir.to_owned();
        let dir: PathBuf = fs::canonicalize(dir).catch_err(&format!(
            "Could not canonicalize path: {}",
            rel_dir.display()
        ))?;

        Ok(Self {
            idx,
            name: dir
                .file_name()
                .catch_err(&format!("Could not parse file name: {}", rel_dir.display()))?
                .to_string_lossy()
                .to_string(),
            dir: dir.to_owned(),
            main: AstMod::parse_dir(
                span_files,
                dir.join("src"),
                &vec!["crate".to_string()],
                if is_entry {
                    DirType::Main
                } else {
                    DirType::Lib
                },
            )?,
            deps: HashMap::new(),
        })
    }

    pub fn parse(mut dir: PathBuf) -> MsgResult<(Crates, SpanFiles)> {
        let mut span_files = SpanFiles::new();

        let mut crates = vec![AstCrate::new(&mut span_files, dir.to_owned(), 0, true)?];

        let engine_dir = get_engine_dir()?;
        let macros_dir = get_macros_dir()?;

        let mut crate_deps = Vec::new();
        let mut i = 0;
        while i < crates.len() {
            let cr_dir = crates.try_get(i)?.dir.to_owned();
            let (deps, new_deps) = Self::get_crate_dependencies(cr_dir.to_owned(), &crates)?;
            crate_deps.push(deps);
            for path in new_deps {
                crates.push(AstCrate::new(
                    &mut span_files,
                    cr_dir.join(path),
                    crates.len(),
                    false,
                )?);
            }
            i += 1;
        }

        let engine_cr_idx = crates
            .iter()
            .find_map(|cr| (cr.dir == engine_dir).then_some(cr.idx))
            .catch_err(&format!(
                "Could not find engine crate: '{}'",
                engine_dir.display()
            ))?;
        let macros_cr_idx = crates
            .iter()
            .find_map(|cr| (cr.dir == macros_dir).then_some(cr.idx))
            .catch_err(&format!(
                "Could not find macros crate: '{}'",
                macros_dir.display()
            ))?;

        // Insert correct dependencies
        for (cr, deps) in crates.iter_mut().zip(crate_deps.into_iter()) {
            cr.deps = deps.into_iter().collect();
        }

        let mut crate_idxs = [0; Crate::LEN];
        crate_idxs[Crate::Main as usize] = 0;
        crate_idxs[Crate::Engine as usize] = engine_cr_idx;
        crate_idxs[Crate::Macros as usize] = macros_cr_idx;

        Ok((Crates::new(crates, crate_idxs)?, span_files))
    }

    fn get_crate_dependencies(
        cr_dir: PathBuf,
        crates: &Vec<AstCrate>,
    ) -> MsgResult<(Vec<(usize, String)>, Vec<String>)> {
        let deps = AstCrate::parse_cargo_toml(cr_dir.to_owned())?;
        let mut new_deps = Vec::new();
        let mut cr_deps = Vec::new();
        for (name, path) in deps {
            let dep_dir = fs::canonicalize(cr_dir.join(path.to_string())).catch_err(&format!(
                "Could not canonicalize dependency path: {}: {}/{}",
                name,
                cr_dir.display(),
                path
            ))?;
            cr_deps.push(match crates.iter().position(|cr| cr.dir == dep_dir) {
                Some(i) => (i, name),
                None => {
                    new_deps.push(path);
                    (crates.len() + new_deps.len() - 1, name)
                }
            });
        }
        Ok((cr_deps, new_deps))
    }

    fn parse_cargo_toml(dir: PathBuf) -> MsgResult<HashMap<String, String>> {
        // Get the path to the `Cargo.toml` file
        let cargo_toml_path = dir.join("Cargo.toml");

        // Read the `Cargo.toml` file into a string
        let mut cargo_toml = String::new();
        let mut file = File::open(&cargo_toml_path).catch_err(&format!(
            "Could not open Cargo.toml: {}",
            cargo_toml_path.display()
        ))?;
        file.read_to_string(&mut cargo_toml).catch_err(&format!(
            "Could not read Cargo.toml: {}",
            cargo_toml_path.display()
        ))?;

        // Parse the `Cargo.toml` file as TOML
        let cargo_toml = cargo_toml.parse::<toml::Value>().catch_err(&format!(
            "Could not parse Cargo.toml: {}",
            cargo_toml_path.display()
        ))?;

        // Extract the list of dependencies from the `Cargo.toml` file
        let deps = cargo_toml
            .get("dependencies")
            .catch_err("Could not find 'dependencies' section in Cargo.toml")?
            .as_table()
            .catch_err("Could not convert 'dependencies' section to a table")?;
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
    fn get_mod_from_path<'a>(&'a mut self, path: &[String]) -> MsgResult<&'a mut AstMod> {
        let mut path_it = path.iter();
        if !path_it.next().is_some_and(|s| s == "crate") {
            return Err(vec![Msg::String(format!(
                "No mod defined at the path: {path:#?}"
            ))]);
        }
        let mut m = &mut self.main;
        while let Some(ident) = path_it.next() {
            m = m
                .mods
                .iter_mut()
                .find(|m| m.path.last().is_some_and(|p| p == ident))
                .catch_err(&format!("No mod defined at the path: {path:#?}"))?;
        }
        Ok(m)
    }

    pub fn add_symbol(&mut self, sym: Symbol) -> MsgResult<()> {
        self.get_mod_from_path(sym.path.slice_to(-1))?
            .symbols
            .push(sym);
        Ok(())
    }

    pub fn add_mod<'a>(&'a mut self, path: Vec<String>) -> MsgResult<&'a mut AstMod> {
        let m = self.get_mod_from_path(path.slice_to(-1))?;
        m.mods.push(AstMod::new(
            m.dir.to_owned(),
            path.to_vec(),
            AstModType::Internal,
            m.span_file,
            m.span_start,
        ));
        m.mods
            .last_mut()
            .catch_err(&format!("Mod not added: {path:#?}"))
    }

    pub fn add_hardcoded_symbol(crates: &mut Crates, sym: HardcodedSymbol) -> MsgResult<()> {
        let path = sym.get_path();
        crates.get_crate_mut(path.cr)?.add_symbol(Symbol {
            kind: SymbolType::Hardcoded(sym),
            path: path.full_path(),
            public: true,
        });
        Ok(())
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
