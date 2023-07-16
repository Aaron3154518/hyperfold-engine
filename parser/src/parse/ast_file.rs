use std::{fs, path::PathBuf};

use syn::visit::Visit;

use super::{ast_mod::AstModType, AstMod};
use crate::utils::{constants::NAMESPACE, SpanFiles};

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
    pub fn parse_mod(span_files: &mut SpanFiles, path: PathBuf, mods: &Vec<String>) -> Self {
        if path.is_dir() {
            Self::parse_dir(span_files, path, mods, DirType::Mod)
        } else {
            let mut f_path = path.to_owned();
            f_path.set_extension("rs");
            if f_path.is_file() {
                Self::parse_file(span_files, path, f_path, mods, AstModType::File)
            } else {
                panic!("File does not exist: {}", (f_path.display()))
            }
        }
    }

    pub fn parse_file(
        span_files: &mut SpanFiles,
        dir: PathBuf,
        path: PathBuf,
        mods: &Vec<String>,
        ty: AstModType,
    ) -> Self {
        Self::parse(span_files, dir, path, mods.to_vec(), ty)
    }

    pub fn parse_dir(
        span_files: &mut SpanFiles,
        path: PathBuf,
        mods: &Vec<String>,
        ty: DirType,
    ) -> Self {
        Self::parse_file(
            span_files,
            path.to_owned(),
            path.join(ty.to_file()),
            mods,
            AstModType::from(ty),
        )
    }
}
