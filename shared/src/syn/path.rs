use diagnostic::CatchErr;

use crate::traits::CollectVecInto;

use super::error::{CriticalResult, StrToError};

// To path
pub fn vec_to_path(path: Vec<String>) -> CriticalResult<syn::Path> {
    string_to_path(path.join("::"))
}

pub fn arr_to_path<const N: usize>(path: [&str; N]) -> CriticalResult<syn::Path> {
    string_to_path(path.join("::"))
}

pub fn string_to_path(path: String) -> CriticalResult<syn::Path> {
    syn::parse_str(&path).catch_err(format!("Could not parse path: {}", path).trace())
}

pub fn path_to_vec(path: &syn::Path) -> Vec<String> {
    path.segments.iter().map_vec_into(|s| s.ident.to_string())
}
