use std::{fs, path::PathBuf};

use diagnostic::DiagnosticResult;
use syn::visit::Visit;

use super::{ast_mod::AstModType, AstMod};
use crate::utils::{constants::NAMESPACE, SpanFiles};
use shared::syn::{DiagnosticResult, Msg};

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

impl From<DirType> for AstModType {
    fn from(value: DirType) -> Self {
        match value {
            DirType::Main => Self::Main,
            DirType::Lib => Self::Lib,
            DirType::Mod => Self::Mod,
        }
    }
}

// Pass 1: parsing
impl AstMod {
    pub fn parse_mod(path: PathBuf, mods: &Vec<String>) -> DiagnosticResult<Self> {
        if path.is_dir() {
            Self::parse_dir(path, mods, DirType::Mod)
        } else {
            let mut f_path = path.to_owned();
            f_path.set_extension("rs");
            if f_path.is_file() {
                Self::parse_file(f_path, mods, AstModType::File)
            } else {
                Err(vec![Error::new(&format!(
                    "File does not exist: {}",
                    f_path.display()
                ))])
            }
        }
    }

    pub fn parse_file(path: PathBuf, mods: &Vec<String>, ty: AstModType) -> DiagnosticResult<Self> {
        Self::parse(path, mods.to_vec(), ty)
    }

    pub fn parse_dir(path: PathBuf, mods: &Vec<String>, ty: DirType) -> DiagnosticResult<Self> {
        Self::parse_file(path.join(ty.to_file()), mods, AstModType::from(ty))
    }
}
