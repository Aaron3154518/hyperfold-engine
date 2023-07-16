use crate::utils::{CatchErr, MsgResult};

// To path
pub fn vec_to_path(path: Vec<String>) -> MsgResult<syn::Path> {
    string_to_path(path.join("::"))
}

pub fn arr_to_path<const N: usize>(path: [&str; N]) -> MsgResult<syn::Path> {
    string_to_path(path.join("::"))
}

pub fn string_to_path(path: String) -> MsgResult<syn::Path> {
    syn::parse_str(&path).catch_err(format!("Could not parse path: {}", path).as_str())
}
