use diagnostic::CombineResults;
use proc_macro2::TokenStream;
use quote::quote;
use shared::{
    syn::{error::CriticalResult, vec_to_path},
    traits::{CollectVec, CollectVecInto},
};

use crate::{
    resolve::ItemGlobal,
    utils::idents::{global_var, CODEGEN_IDENTS},
};

use super::Crates;

struct CodegenArgs<'a> {
    struct_name: &'a syn::Ident,
    vars: Vec<syn::Ident>,
    types: Vec<syn::Path>,
}

fn codegen(
    CodegenArgs {
        struct_name,
        vars,
        types,
    }: CodegenArgs,
) -> TokenStream {
    quote!(
        struct #struct_name {
            #(#vars: #types),*
        }

        impl #struct_name {
            fn new() -> Self {
                Self {
                    #(#vars: #types::new()),*
                }
            }
        }
    )
}

pub fn globals(
    cr_idx: usize,
    globals: &Vec<ItemGlobal>,
    crates: &Crates,
) -> CriticalResult<TokenStream> {
    let vars = (0..globals.len()).map_vec_into(|i| global_var(i));
    let types = globals
        .map_vec(|g| crates.get_item_syn_path(cr_idx, &g.data.path))
        .combine_results();

    types.map(|types| {
        codegen(CodegenArgs {
            struct_name: &CODEGEN_IDENTS.globals,
            vars,
            types,
        })
    })
}
