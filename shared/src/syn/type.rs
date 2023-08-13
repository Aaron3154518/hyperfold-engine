use quote::ToTokens;

use super::error::{err, BuildResult, CatchErr};

// To type
pub fn type_to_type(ty: &syn::Type, r: bool, m: bool) -> BuildResult<syn::Type> {
    string_to_type(ty.to_token_stream().to_string(), r, m)
}

pub fn arr_to_type<const N: usize>(path: [&str; N], r: bool, m: bool) -> BuildResult<syn::Type> {
    string_to_type(path.join("::"), r, m)
}

pub fn string_to_type(ty: String, r: bool, m: bool) -> BuildResult<syn::Type> {
    syn::parse_str::<syn::Type>(
        format!(
            "{}{}{}",
            if r { "&" } else { "" },
            if m { "mut " } else { "" },
            ty
        )
        .as_str(),
    )
    .catch_err(err("Could not parse type"))
}

// Get type generics
pub fn get_type_generics(ty: &syn::TypePath) -> Option<Vec<&syn::GenericArgument>> {
    ty.path.segments.last().and_then(|s| match &s.arguments {
        syn::PathArguments::AngleBracketed(ab) => Some(ab.args.iter().collect::<Vec<_>>()),
        _ => None,
    })
}
