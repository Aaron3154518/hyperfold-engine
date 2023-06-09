use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use shared::util::{JoinMap, JoinMapInto};

use crate::{
    codegen2::{idents::CodegenIdents, util::vec_to_path, CODEGEN_IDENTS},
    match_ok,
    resolve::{
        util::{MsgsResult, Zip2Msgs},
        Crate, CratePath, ItemPath,
    },
};

use super::Crates;

pub fn trait_defs<T, F>(
    cr_idx: usize,
    crates: &Crates,
    items: &Vec<T>,
    get_item_path: F,
    trait_ident: &syn::Ident,
    trait_source: &CratePath,
) -> MsgsResult<TokenStream>
where
    F: for<'a> Fn(&'a T) -> &'a ItemPath,
{
    let macro_cr_idx = crates.get_crate_index(Crate::Macros);

    let CodegenIdents { namespace, .. } = &*CODEGEN_IDENTS;
    let trait_source = crates.get_syn_path(cr_idx, trait_source);

    // Events for this crate
    let types = items.filter_map_vec(|t| {
        let path = get_item_path(t);
        (cr_idx == path.cr_idx).then(|| vec_to_path(path.path.to_vec()))
    });
    // Event traits for dependency crates
    let macro_cr_idx = crates.get_crate_index(Crate::Macros);
    let dep_traits = crates
        .get(cr_idx)
        .ok_or(vec![format!("Invalid crate index: {cr_idx}")])
        .map(|cr| {
            cr.deps
                .iter()
                .filter(|(idx, _)| idx != &&macro_cr_idx)
                .map_vec_into(|(idx, alias)| {
                    let alias = format_ident!("{alias}");
                    quote!(#alias::#namespace::#trait_ident)
                })
        });

    match_ok!(Zip2Msgs, trait_source, dep_traits, {
        let traits = [dep_traits, types.map_vec(|ty| quote!(#trait_source<#ty>))].concat();
        match traits.split_first() {
            Some((first, tail)) => {
                quote!(
                    pub trait #trait_ident: #first #(+#tail)* {}
                )
            }
            None => quote!(pub trait #trait_ident {}),
        }
    })
}

pub struct Traits {
    pub add_event: TokenStream,
    pub add_component: TokenStream,
}
