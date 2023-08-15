use std::{fs, path::PathBuf};

use diagnostic::ToErr;
use shared::syn::error::MsgResult;
use syn::visit::Visit;

use super::{ast_mod::AstModType, AstMod};
use crate::utils::{constants::NAMESPACE, tree::Tree};

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
    pub fn parse_mod(path: PathBuf, mod_path: &Vec<String>) -> MsgResult<Tree<Self>> {
        if path.is_dir() {
            Self::parse_dir(path, mod_path, DirType::Mod)
        } else {
            let mut f_path = path.to_owned();
            f_path.set_extension("rs");
            if f_path.is_file() {
                Self::parse_file(f_path, mod_path, AstModType::File)
            } else {
                format!("File does not exist: {}", f_path.display()).as_err()
            }
        }
    }

    pub fn parse_file(
        path: PathBuf,
        mod_path: &Vec<String>,
        ty: AstModType,
    ) -> MsgResult<Tree<Self>> {
        Self::parse(path, mod_path.to_vec(), ty)
    }

    pub fn parse_dir(path: PathBuf, mod_path: &Vec<String>, ty: DirType) -> MsgResult<Tree<Self>> {
        Self::parse_file(path.join(ty.to_file()), mod_path, AstModType::from(ty))
    }
}
