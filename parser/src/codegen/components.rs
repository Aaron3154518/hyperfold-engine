use proc_macro2::TokenStream;
use quote::quote;
use std::array;

use shared::{
    match_ok,
    msg_result::{CombineMsgs, Zip5Msgs},
    traits::{AndThen, CollectVec, CollectVecInto, ThenOk},
};

use crate::{
    resolve::ItemComponent,
    utils::{
        idents::{component_var, CodegenIdents, CODEGEN_IDENTS},
        paths::{Crate, ENGINE_GLOBALS, ENGINE_PATHS, ENGINE_TRAITS},
        syn::vec_to_path,
        Msg, MsgResult,
    },
};

use super::{traits::trait_defs, Crates};

struct CodegenArgs<'a> {
    struct_name: &'a syn::Ident,
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
    let [mut tys, mut news, mut adds, mut appends, mut removes] = array::from_fn(|_| Vec::new());
    for (i, (c, ty)) in components.iter().zip(types).enumerate() {
        let var = component_var(i);
        if c.args.is_singleton {
            tys.push(quote!(#singleton<#ty>));
            news.push(quote!(#singleton::None));
            adds.push(quote!(self.#var = #singleton::new(e, t)));
            appends.push(quote!(
                if !self.#var.set(&mut cm.#var) {
                    panic!("Cannot set '{}' component more than once as it is marked Singleton",
                        self.#var.typename())
                }
            ));
            removes.push(quote!(self.#var.remove(&eid);));
        } else {
            tys.push(quote!(#entity_map<#ty>));
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
            #(#vars: #tys),*
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

pub fn components(
    cr_idx: usize,
    components: &Vec<ItemComponent>,
    crates: &Crates,
) -> MsgResult<TokenStream> {
    let vars = (0..components.len()).map_vec_into(|i| component_var(i));
    let types = components
        .map_vec(|c| crates.get_item_syn_path(cr_idx, &c.path))
        .combine_msgs();

    let entity_set = crates.get_syn_path(cr_idx, &ENGINE_PATHS.entity_set);
    let entity_trash = crates.get_syn_path(cr_idx, &ENGINE_GLOBALS.entity_trash);
    let entity_map = crates.get_syn_path(cr_idx, &ENGINE_PATHS.entity_map);
    let singleton = crates.get_syn_path(cr_idx, &ENGINE_PATHS.singleton);

    match_ok!(
        Zip5Msgs,
        types,
        entity_set,
        entity_trash,
        entity_map,
        singleton,
        {
            codegen(CodegenArgs {
                struct_name: &CODEGEN_IDENTS.components,
                components,
                types,
                entity_set,
                entity_trash,
                entity_map,
                singleton,
            })
        }
    )
}

pub fn component_trait_defs(
    cr_idx: usize,
    components: &Vec<ItemComponent>,
    crates: &Crates,
) -> MsgResult<TokenStream> {
    trait_defs(
        cr_idx,
        crates,
        components,
        |c| &c.path,
        &CODEGEN_IDENTS.add_component,
        &ENGINE_TRAITS.add_component,
    )
}

pub fn component_trait_impls(
    cr_idx: usize,
    components: &Vec<ItemComponent>,
    crates: &Crates,
) -> MsgResult<TokenStream> {
    let macro_cr_idx = crates.get_crate_index(Crate::Macros);

    let types = components
        .map_vec(|c| crates.get_item_syn_path(cr_idx, &c.path))
        .combine_msgs();
    let crate_paths = crates
        .get_crate_syn_paths(cr_idx, [macro_cr_idx])
        .map(|paths| paths.map_vec_into(|(_, p)| p));

    let CodegenIdents {
        namespace,
        components: components_type,
        add_component,
        ..
    } = &*CODEGEN_IDENTS;
    let add_comp_trait = crates.get_syn_path(cr_idx, &ENGINE_TRAITS.add_component);
    let entity = crates.get_syn_path(cr_idx, &ENGINE_PATHS.entity);
    let singleton = crates.get_syn_path(cr_idx, &ENGINE_PATHS.singleton);

    match_ok!(
        Zip5Msgs,
        types,
        crate_paths,
        add_comp_trait,
        entity,
        singleton,
        {
            let mut adds = Vec::new();
            for (i, c) in components.iter().enumerate() {
                let var = component_var(i);
                adds.push(if c.args.is_singleton {
                    quote!(self.#var = #singleton::new(e, t))
                } else {
                    quote!(self.#var.insert(e, t);)
                });
            }
            quote!(
                #(
                    impl #add_comp_trait<#types> for #components_type {
                        fn add_component(&mut self, e: #entity, t: #types) {
                            self.eids.insert(e);
                            #adds
                        }
                    }
                )*
                #(
                    impl #crate_paths::#namespace::#add_component for #components_type {}
                )*
            )
        }
    )
}
