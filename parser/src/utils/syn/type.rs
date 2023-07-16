use quote::ToTokens;

use crate::utils::{CatchErr, MsgResult};

// To type
pub fn type_to_type(ty: &syn::Type, r: bool, m: bool) -> MsgResult<syn::Type> {
    string_to_type(ty.to_token_stream().to_string(), r, m)
}

pub fn arr_to_type<const N: usize>(path: [&str; N], r: bool, m: bool) -> MsgResult<syn::Type> {
    string_to_type(path.join("::"), r, m)
}

pub fn string_to_type(ty: String, r: bool, m: bool) -> MsgResult<syn::Type> {
    syn::parse_str::<syn::Type>(
        format!(
            "{}{}{}",
            if r { "&" } else { "" },
            if m { "mut " } else { "" },
            ty
        )
        .as_str(),
    )
    .catch_err("Could not parse type")
}
