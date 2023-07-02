use std::array;

use proc_macro2::TokenStream;
use quote::quote;
use shared::util::{JoinMap, JoinMapInto};

use crate::{
    codegen2::idents::CodegenIdents,
    match_ok,
    resolve::{
        constants::component_var,
        util::{CombineMsgs, MsgsResult, ToMsgsResult, Zip5Msgs},
        EngineGlobals, EnginePaths, GetPaths, ItemComponent,
    },
};

use super::{util::vec_to_path, Codegen, Crates};

struct CodegenArgs<'a> {
    struct_name: syn::Ident,
    components: &'a Vec<ItemComponent>,
    types: Vec<syn::Path>,
    entity_set: syn::Path,
    entity_trash: syn::Path,
    entity_map: syn::Path,
    singleton: syn::Path,
}

fn codegen<'a>(
    CodegenArgs {
        struct_name,
        components,
        types,
        entity_set,
        entity_trash,
        entity_map,
        singleton,
    }: CodegenArgs<'a>,
) -> TokenStream {
    let mut vars = Vec::new();
    let [mut news, mut adds, mut appends, mut removes] = array::from_fn(|_| Vec::new());
    for (i, c) in components.iter().enumerate() {
        let var = component_var(i);
        if c.args.is_singleton {
            news.push(quote!(None));
            adds.push(quote!(self.#var = Some(#singleton::new(e, t))));
            appends.push(quote!(
                if cm.#var.is_some() {
                    if self.#var.is_some() {
                        panic!("Cannot set Singleton component more than once")
                    }
                    self.#var = cm.#var.take();
                }
            ));
            removes.push(quote!(
                if self.#var.as_ref().is_some_and(|s| s.contains_key(&eid)) {
                    self.#var = None;
                }
            ));
        } else {
            news.push(quote!(#entity_map::new()));
            adds.push(quote!(self.#var.insert(e, t);));
            appends.push(quote!(self.#var.extend(cm.#var.drain());));
            removes.push(quote!(self.#var.remove(&eid);));
        }
        vars.push(var);
    }

    quote!(
        pub struct #struct_name {
            eids: #entity_set,
            #(#vars: #types),*
        }

        impl #struct_name {
            fn new() -> Self {
                Self {
                    eids: #entity_set::new(),
                    #(#vars: #news),*
                }
            }

            fn append(&mut self, cm: &mut Self) {
                self.eids.extend(cm.eids.drain());
                #(#appends)*
            }

            fn remove(&mut self, tr: &mut #entity_trash) {
                for eid in tr.0.drain(..) {
                    self.eids.remove(&eid);
                    #(#removes)*
                }
            }
        }
    )
}

impl Codegen {
    pub fn components(
        cr_idx: usize,
        components: &Vec<ItemComponent>,
        crates: &Crates,
    ) -> MsgsResult<TokenStream> {
        let struct_name = CodegenIdents::CFoo.to_ident();
        let vars = (0..components.len()).map_vec(|i| component_var(i));
        let types = components
            .map_vec(|c| crates.get_path(cr_idx, &c.path).map(|v| vec_to_path(v)))
            .combine_msgs();

        let entity_set = crates
            .path_from(cr_idx, EnginePaths::EntitySet)
            .map(|v| vec_to_path(v))
            .to_msg_vec();
        let entity_trash = crates
            .path_from(cr_idx, EngineGlobals::EntityTrash)
            .map(|v| vec_to_path(v))
            .to_msg_vec();
        let entity_map = crates
            .path_from(cr_idx, EnginePaths::EntityMap)
            .map(|v| vec_to_path(v))
            .to_msg_vec();
        let singleton = crates
            .path_from(cr_idx, EnginePaths::Singleton)
            .map(|v| vec_to_path(v))
            .to_msg_vec();

        match_ok!(types, entity_set, entity_trash, entity_map, singleton, {
            codegen(CodegenArgs {
                struct_name,
                components,
                types,
                entity_set,
                entity_trash,
                entity_map,
                singleton,
            })
        })
    }
}
