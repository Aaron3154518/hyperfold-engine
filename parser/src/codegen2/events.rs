use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use shared::util::{JoinMap, JoinMapInto};

use crate::{
    codegen2::idents::CodegenIdents,
    match_ok,
    resolve::{
        constants::{event_var, event_variant},
        util::{CombineMsgs, MsgsResult, Zip2Msgs, Zip3Msgs, Zip5Msgs},
        Crate, EnginePaths, EngineTraits, ItemEvent,
    },
};

use super::{traits::trait_defs, util::vec_to_path, Crates};

pub fn events_enum(events: &Vec<ItemEvent>) -> TokenStream {
    let enum_name = CodegenIdents::E.to_ident();
    let num_variants_var = CodegenIdents::ELen.to_ident();
    let num_variants = events.len();
    let variants = (0..events.len()).map_vec_into(|i| event_variant(i));

    quote!(
        #[derive(Hash, Clone, Copy, Eq, PartialEq, Debug)]
        enum #enum_name {
            #(#variants),*
        }
        const #num_variants_var: usize = #num_variants;
    )
}

pub fn events(cr_idx: usize, events: &Vec<ItemEvent>, crates: &Crates) -> MsgsResult<TokenStream> {
    let struct_name = CodegenIdents::EFoo.to_ident();
    let enum_name = CodegenIdents::E.to_ident();
    let (vars, variants) = (0..events.len()).unzip_vec_into(|i| (event_var(i), event_variant(i)));
    let types = events
        .map_vec(|e| {
            crates
                .get_item_path(cr_idx, &e.path)
                .map(|v| vec_to_path(v))
        })
        .combine_msgs();

    types.map(|types| {
        quote!(
            pub struct #struct_name {
                #(#vars: Vec<#types>),*,
                events: std::collections::VecDeque<(#enum_name, usize)>
            }

            impl #struct_name {
                fn new() -> Self {
                    Self {
                        #(#vars: Vec::new()),*,
                        events: std::collections::VecDeque::new()
                    }
                }

                fn has_events(&self) -> bool {
                    !self.events.is_empty()
                }

                fn add_event(&mut self, e: #enum_name) {
                    self.events.push_back((e, 0));
                }

                fn get_events(&mut self) -> std::collections::VecDeque<(#enum_name, usize)> {
                    std::mem::replace(&mut self.events, std::collections::VecDeque::new())
                }

                fn append(&mut self, other: &mut Self) {
                    #(
                        other.#vars.reverse();
                        self.#vars.append(&mut other.#vars);
                    )*
                }

                fn pop(&mut self, e: #enum_name) {
                    match e {
                        #(
                            #enum_name::#variants => {
                                self.#vars.pop();
                            }
                        )*
                    }
                }
            }
        )
    })
}

pub fn event_trait_defs(
    cr_idx: usize,
    events: &Vec<ItemEvent>,
    crates: &Crates,
) -> MsgsResult<TokenStream> {
    trait_defs(
        cr_idx,
        crates,
        events,
        |e| &e.path,
        CodegenIdents::AddEvent.to_ident(),
        EngineTraits::AddEvent,
    )
}

pub fn event_trait_impls(
    cr_idx: usize,
    events: &Vec<ItemEvent>,
    crates: &Crates,
) -> MsgsResult<TokenStream> {
    let macro_cr_idx = crates.get_crate_index(Crate::Macros);

    let (vars, variants) = (0..events.len()).unzip_vec_into(|i| (event_var(i), event_variant(i)));

    // Implement trait for every event
    let types = events
        .map_vec(|c| {
            crates
                .get_item_path(cr_idx, &c.path)
                .map(|v| vec_to_path(v))
        })
        .combine_msgs();
    // Implement all dependency traits
    let crate_paths = crates
        .get_crate_paths(cr_idx, [macro_cr_idx])
        .map(|v| v.into_iter().map_vec_into(|(i, path)| vec_to_path(path)))
        .ok_or(vec![format!("Invalid crate index: {cr_idx}")]);

    let struct_name = CodegenIdents::EFoo.to_ident();
    let enum_name = CodegenIdents::E.to_ident();
    let namespace = CodegenIdents::Namespace.to_ident();
    let add_event = CodegenIdents::AddEvent.to_ident();
    let add_event_trait = crates.get_syn_path(cr_idx, EngineTraits::AddEvent);

    match_ok!(Zip3Msgs, types, crate_paths, add_event_trait, {
        quote!(
            #(
                impl #add_event_trait<#types> for #struct_name {
                    fn new_event(&mut self, t: #types) {
                        self.#vars.push(t);
                        self.add_event(#enum_name::#variants);
                    }

                    fn get_event<'a>(&'a self) -> Option<&'a #types> {
                        self.#vars.last()
                    }
                }
            )*
            #(
                impl #crate_paths::#namespace::#add_event for #struct_name {}
            )*
        )
    })
}
