use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use shared::{
    match_ok,
    msg_result::{CombineMsgs, MsgResult, Zip3Msgs},
    traits::{CollectVec, CollectVecInto},
};

use crate::{
    resolve::ItemEvent,
    utils::{
        idents::{event_var, event_variant, CodegenIdents, CODEGEN_IDENTS},
        paths::{Crate, ENGINE_TRAITS},
        syn::vec_to_path,
    },
};

use super::{traits::trait_defs, Crates};

pub fn events_enum(events: &Vec<ItemEvent>) -> TokenStream {
    let CodegenIdents {
        event_enum,
        event_enum_len,
        ..
    } = &*CODEGEN_IDENTS;

    let num_variants = events.len();
    let variants = (0..events.len()).map_vec_into(|i| event_variant(i));

    quote!(
        #[derive(Hash, Clone, Copy, Eq, PartialEq, Debug)]
        enum #event_enum {
            #(#variants),*
        }
        const #event_enum_len: usize = #num_variants;
    )
}

pub fn events(cr_idx: usize, events: &Vec<ItemEvent>, crates: &Crates) -> MsgResult<TokenStream> {
    let CodegenIdents {
        events: events_type,
        event_enum,
        ..
    } = &*CODEGEN_IDENTS;

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
            pub struct #events_type {
                #(#vars: Vec<#types>),*,
                events: std::collections::VecDeque<(#event_enum, usize)>
            }

            impl #events_type {
                fn new() -> Self {
                    Self {
                        #(#vars: Vec::new()),*,
                        events: std::collections::VecDeque::new()
                    }
                }

                fn has_events(&self) -> bool {
                    !self.events.is_empty()
                }

                fn add_event(&mut self, e: #event_enum) {
                    self.events.push_back((e, 0));
                }

                fn get_events(&mut self) -> std::collections::VecDeque<(#event_enum, usize)> {
                    std::mem::replace(&mut self.events, std::collections::VecDeque::new())
                }

                fn append(&mut self, other: &mut Self) {
                    #(
                        other.#vars.reverse();
                        self.#vars.append(&mut other.#vars);
                    )*
                }

                fn pop(&mut self, e: #event_enum) {
                    match e {
                        #(
                            #event_enum::#variants => {
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
) -> MsgResult<TokenStream> {
    trait_defs(
        cr_idx,
        crates,
        events,
        |e| &e.path,
        &CODEGEN_IDENTS.add_event,
        &ENGINE_TRAITS.add_event,
    )
}

pub fn event_trait_impls(
    cr_idx: usize,
    events: &Vec<ItemEvent>,
    crates: &Crates,
) -> MsgResult<TokenStream> {
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

    let CodegenIdents {
        events: events_type,
        event_enum,
        namespace,
        add_event,
        ..
    } = &*CODEGEN_IDENTS;
    let add_event_trait = crates.get_syn_path(cr_idx, &ENGINE_TRAITS.add_event);

    match_ok!(Zip3Msgs, types, crate_paths, add_event_trait, {
        quote!(
            #(
                impl #add_event_trait<#types> for #events_type {
                    fn new_event(&mut self, t: #types) {
                        self.#vars.push(t);
                        self.add_event(#event_enum::#variants);
                    }

                    fn get_event<'a>(&'a self) -> Option<&'a #types> {
                        self.#vars.last()
                    }
                }
            )*
            #(
                impl #crate_paths::#namespace::#add_event for #events_type {}
            )*
        )
    })
}
