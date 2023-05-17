use std::{fs, path::PathBuf};

use syn::visit::Visit;

use super::ast_mod::{Mod, ModType};
use crate::validate::constants::NAMESPACE;

#[derive(Debug)]
pub enum DirType {
    Main,
    Lib,
    Mod,
}

impl DirType {
    pub fn to_file(&self) -> &str {
        match self {
            DirType::Main => "main.rs",
            DirType::Lib => "lib.rs",
            DirType::Mod => "mod.rs",
        }
    }
}

impl From<DirType> for ModType {
    fn from(value: DirType) -> Self {
        match value {
            DirType::Main => Self::Main,
            DirType::Lib => Self::Lib,
            DirType::Mod => Self::Mod,
        }
    }
}

// Pass 1: parsing
impl Mod {
    pub fn parse_mod(path: PathBuf, mods: &Vec<String>) -> Self {
        if path.is_dir() {
            Self::parse_dir(path, mods, DirType::Mod)
        } else {
            let mut f_path = path.to_owned();
            f_path.set_extension("rs");
            if f_path.is_file() {
                Self::parse_file(path, f_path, mods, ModType::File)
            } else {
                panic!("File does not exist: {}", f_path.display())
            }
        }
    }

    pub fn parse_file(dir: PathBuf, path: PathBuf, mods: &Vec<String>, ty: ModType) -> Self {
        let mut file_mod = Self::new(dir, mods.to_vec(), ty);
        file_mod.parse(path);
        file_mod
    }

    pub fn parse_dir(path: PathBuf, mods: &Vec<String>, ty: DirType) -> Self {
        Self::parse_file(
            path.to_owned(),
            path.join(ty.to_file()),
            mods,
            ModType::from(ty),
        )
    }
}
