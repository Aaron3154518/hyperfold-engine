use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

// To type
pub fn type_to_type(ty: &syn::Type, r: bool, m: bool) -> syn::Type {
    string_to_type(ty.to_token_stream().to_string(), r, m)
}

pub fn arr_to_type<const N: usize>(path: [&str; N], r: bool, m: bool) -> syn::Type {
    string_to_type(path.join("::"), r, m)
}

pub fn string_to_type(ty: String, r: bool, m: bool) -> syn::Type {
    syn::parse_str::<syn::Type>(
        format!(
            "{}{}{}",
            if r { "&" } else { "" },
            if m { "mut " } else { "" },
            ty
        )
        .as_str(),
    )
    .expect("Could not parse type")
}

// To path
pub fn vec_to_path(path: Vec<String>) -> syn::Path {
    string_to_path(path.join("::"))
}

pub fn arr_to_path<const N: usize>(path: [&str; N]) -> syn::Path {
    string_to_path(path.join("::"))
}

pub fn string_to_path(path: String) -> syn::Path {
    syn::parse_str(&path).expect(format!("Could not parse path: {}", path).as_str())
}

// Convert ident to TokenStream
pub trait Quote {
    fn quote(&self) -> TokenStream;
}

impl Quote for syn::Ident {
    fn quote(&self) -> TokenStream {
        quote!(#self)
    }
}
