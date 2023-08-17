use diagnostic::{zip_match, CombineResults, ToErr, ZipResults};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use shared::{
    syn::{error::MsgResult, vec_to_path},
    traits::{CollectVec, CollectVecInto},
};

use crate::{
    parse::ItemPath,
    utils::{
        idents::{CodegenIdents, CODEGEN_IDENTS},
        paths::{Crate, CratePath},
    },
};

use super::Crates;

pub trait GetTraitTypes {
    fn get_paths(&self) -> Vec<&ItemPath>;
}

pub fn trait_defs<const N: usize>(
    cr_idx: usize,
    crates: &Crates,
    trait_ident: &syn::Ident,
    item_traits: [(&CratePath, &dyn GetTraitTypes); N],
) -> MsgResult<TokenStream> {
    let macro_cr_idx = crates.get_crate_index(Crate::Macros);

    let CodegenIdents { namespace, .. } = &*CODEGEN_IDENTS;
    let item_traits = item_traits
        .map_vec_into(|(tr, items)| {
            let tr = crates.get_syn_path(cr_idx, tr);
            let paths = items
                .get_paths()
                .filter_map_vec_into(|p| (cr_idx == p.cr_idx).then(|| vec_to_path(p.path.to_vec())))
                .combine_results();
            zip_match!((tr, paths) => {
                paths.map_vec_into(|p| quote!(#tr<#p>))
            })
        })
        .combine_results();
    // Event traits for dependency crates
    let macro_cr_idx = crates.get_crate_index(Crate::Macros);
    let mut dep_traits = crates
        .get(cr_idx)
        .ok_or(format!("Invalid crate index: {cr_idx}").as_vec())
        .map(|cr| {
            cr.deps
                .iter()
                .filter(|(idx, _)| idx != &&macro_cr_idx)
                .map_vec_into(|(idx, alias)| {
                    let alias = format_ident!("{alias}");
                    quote!(#alias::#namespace::#trait_ident)
                })
        });

    zip_match!((dep_traits, item_traits) => {
        let mut traits = dep_traits
            .into_iter()
            .chain(item_traits.into_iter().flatten());
        match traits.next() {
            Some(first) => {
                quote!(
                    pub trait #trait_ident: #first #(+#traits)* {}
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
