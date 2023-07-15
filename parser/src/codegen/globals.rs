use proc_macro2::TokenStream;
use quote::quote;
use shared::traits::{CollectVec, CollectVecInto};

use crate::{
    codegen::{util::vec_to_path, Crates},
    match_ok,
    resolve::{
        constants::global_var,
        util::{CombineMsgs, MsgResult},
        ItemGlobal,
    },
    utils::{CodegenIdents, CODEGEN_IDENTS},
};

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
        pub struct #struct_name {
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
) -> MsgResult<TokenStream> {
    let vars = (0..globals.len()).map_vec_into(|i| global_var(i));
    let types = globals
        .map_vec(|g| {
            crates
                .get_item_path(cr_idx, &g.path)
                .map(|v| vec_to_path(v))
        })
        .combine_msgs();

    match_ok!(types, {
        codegen(CodegenArgs {
            struct_name: &CODEGEN_IDENTS.globals,
            vars,
            types,
        })
    })
}
