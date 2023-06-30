use proc_macro2::TokenStream;
use quote::quote;
use shared::util::{JoinMap, JoinMapInto};

use crate::{
    codegen2::{idents::CodegenIdents, util::vec_to_path},
    resolve::{
        constants::global_var,
        util::{CombineMsgs, MsgsResult},
        ItemGlobal,
    },
};

use super::{Codegen, CratePaths};

impl Codegen {
    pub fn globals(
        cr_idx: usize,
        globals: &Vec<ItemGlobal>,
        paths: &CratePaths,
    ) -> MsgsResult<TokenStream> {
        let struct_name = CodegenIdents::GFoo.to_ident();
        let vars = (0..globals.len()).map_vec(|i| global_var(i));
        let types = globals
            .map_vec(|g| {
                paths
                    .prepend_path(cr_idx, g.path.cr_idx, &g.path.path)
                    .map_or(
                        Err(format!(
                            "No path from crate {cr_idx} to crate {}",
                            g.path.cr_idx
                        )),
                        |v| Ok(vec_to_path(v)),
                    )
            })
            .combine_msgs();

        types.map(|types| {
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
        })
    }
}
