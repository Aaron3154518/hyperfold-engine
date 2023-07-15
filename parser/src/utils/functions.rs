use proc_macro2::TokenStream;
use quote::{format_ident, quote};

// Returns `mut` or ``
pub fn get_mut(is_mut: bool) -> TokenStream {
    match is_mut {
        true => quote!(mut),
        false => quote!(),
    }
}

// Returns `{ident}` if not mut, otherwise `{ident}_mut`
pub fn get_fn_name(ident: &str, is_mut: bool) -> syn::Ident {
    match is_mut {
        true => format_ident!("{ident}_mut"),
        false => format_ident!("{ident}"),
    }
}
