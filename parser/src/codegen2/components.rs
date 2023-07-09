use std::array;

use proc_macro2::TokenStream;
use quote::quote;
use shared::util::{AndThen, JoinMap, JoinMapInto, ThenOk};

use crate::{
    codegen2::idents::CodegenIdents,
    match_ok,
    resolve::{
        constants::component_var,
        util::{CombineMsgs, MsgsResult, Zip2Msgs, Zip5Msgs},
        Crate, EngineGlobals, EnginePaths, EngineTraits, GetPaths, ItemComponent,
    },
};

use super::{traits::trait_defs, util::vec_to_path, Crates};

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

pub fn components(
    cr_idx: usize,
    components: &Vec<ItemComponent>,
    crates: &Crates,
) -> MsgsResult<TokenStream> {
    let struct_name = CodegenIdents::CFooType.to_ident();
    let vars = (0..components.len()).map_vec_into(|i| component_var(i));
    let types = components
        .map_vec(|c| {
            crates
                .get_item_path(cr_idx, &c.path)
                .map(|v| vec_to_path(v))
        })
        .combine_msgs();

    let entity_set = crates.get_syn_path(cr_idx, EnginePaths::EntitySet);
    let entity_trash = crates.get_syn_path(cr_idx, EngineGlobals::EntityTrash);
    let entity_map = crates.get_syn_path(cr_idx, EnginePaths::EntityMap);
    let singleton = crates.get_syn_path(cr_idx, EnginePaths::Singleton);

    match_ok!(
        Zip5Msgs,
        types,
        entity_set,
        entity_trash,
        entity_map,
        singleton,
        {
            codegen(CodegenArgs {
                struct_name,
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
) -> MsgsResult<TokenStream> {
    trait_defs(
        cr_idx,
        crates,
        components,
        |c| &c.path,
        CodegenIdents::AddComponent.to_ident(),
        EngineTraits::AddComponent,
    )
}

pub fn component_trait_impls(
    cr_idx: usize,
    components: &Vec<ItemComponent>,
    crates: &Crates,
) -> MsgsResult<TokenStream> {
    let macro_cr_idx = crates.get_crate_index(Crate::Macros);

    let types = components
        .map_vec(|c| {
            crates
                .get_item_path(cr_idx, &c.path)
                .map(|v| vec_to_path(v))
        })
        .combine_msgs();
    let crate_paths = crates
        .get_crate_paths(cr_idx, [macro_cr_idx])
        .map(|v| v.into_iter().map_vec_into(|(i, path)| vec_to_path(path)))
        .ok_or(vec![format!("Invalid crate index: {cr_idx}")]);

    let struct_name = CodegenIdents::CFooType.to_ident();
    let namespace = CodegenIdents::Namespace.to_ident();
    let add_comp = CodegenIdents::AddComponent.to_ident();
    let add_comp_trait = crates.get_syn_path(cr_idx, EngineTraits::AddComponent);
    let entity = crates.get_syn_path(cr_idx, EnginePaths::Entity);
    let singleton = crates.get_syn_path(cr_idx, EnginePaths::Singleton);

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
                    quote!(self.#var = Some(#singleton::new(e, t)))
                } else {
                    quote!(self.#var.insert(e, t);)
                });
            }
            quote!(
                #(
                    impl #add_comp_trait<#types> for #struct_name {
                        fn add_component(&mut self, e: #entity, t: #types) {
                            self.eids.insert(e);
                            #adds
                        }
                    }
                )*
                #(
                    impl #crate_paths::#namespace::#add_comp for #struct_name {}
                )*
            )
        }
    )
}
