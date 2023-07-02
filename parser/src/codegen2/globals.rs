use proc_macro2::TokenStream;
use quote::quote;
use shared::util::{JoinMap, JoinMapInto};

use crate::{
    codegen2::{idents::CodegenIdents, util::vec_to_path},
    match_ok,
    resolve::{
        constants::global_var,
        util::{CombineMsgs, MsgsResult},
        ItemGlobal,
    },
};

use super::{Codegen, Crates};

struct CodegenArgs {
    struct_name: syn::Ident,
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
            pub new() -> Self {
                Self {
                    #(#vars: #types::new()),*
                }
            }
        }
    )
}

impl Codegen {
    pub fn globals(
        cr_idx: usize,
        globals: &Vec<ItemGlobal>,
        crates: &Crates,
    ) -> MsgsResult<TokenStream> {
        let struct_name = CodegenIdents::GFoo.to_ident();
        let vars = (0..globals.len()).map_vec(|i| global_var(i));
        let types = globals
            .map_vec(|g| crates.get_path(cr_idx, &g.path).map(|v| vec_to_path(v)))
            .combine_msgs();

        match_ok!(types, {
            codegen(CodegenArgs {
                struct_name,
                vars,
                types,
            })
        })
    }
}
