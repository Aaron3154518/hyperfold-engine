use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use shared::{
    constants::{STATE_DATA, STATE_ENTER_EVENT, STATE_EXIT_EVENT},
    match_ok,
    msg_result::{CombineMsgs, Zip2Msgs, Zip5Msgs},
    syn::{error::MsgResult, vec_to_path},
    traits::{unzip::Unzip3, CollectVec, CollectVecInto, MapNone},
};

use crate::{
    parse::ItemPath,
    resolve::{ItemEvent, ItemState},
    utils::{
        idents::{
            event_var, event_variant, state_var, state_variant, CodegenIdents, CODEGEN_IDENTS,
        },
        paths::{Crate, ENGINE_TRAITS},
    },
};

use super::{
    traits::{trait_defs, GetTraitTypes},
    Crates,
};

pub fn events_enums(events: &Vec<ItemEvent>, states: &Vec<ItemState>) -> TokenStream {
    let CodegenIdents {
        event_enum,
        event_enum_len,
        state_enum,
        ..
    } = &*CODEGEN_IDENTS;

    let num_variants = events.len();
    let e_variants = (0..events.len()).map_vec_into(|i| event_variant(i));

    let (s_variants, s_enter_events, s_exit_events) = states.enumer_unzipn_vec(
        |(i, s)| {
            (
                state_variant(i),
                event_variant(s.enter_event),
                event_variant(s.exit_event),
            )
        },
        Unzip3::unzip3_vec,
    );

    quote!(
        #[derive(Hash, Clone, Copy, Eq, PartialEq, Debug)]
        enum #state_enum {
            #(#s_variants),*
        }

        #[derive(Hash, Clone, Copy, Eq, PartialEq, Debug)]
        enum #event_enum {
            #(#e_variants),*
        }

        const #event_enum_len: usize = #num_variants;

        impl #event_enum {
            fn enters_state(&self) -> Option<#state_enum> {
                match self {
                    #(#event_enum::#s_enter_events => Some(#state_enum::#s_variants),)*
                    _ => None,
                }
            }

            fn exits_state(&self) -> Option<#state_enum> {
                match self {
                    #(#event_enum::#s_exit_events => Some(#state_enum::#s_variants),)*
                    _ => None,
                }
            }
        }
    )
}

pub fn events(
    cr_idx: usize,
    events: &Vec<ItemEvent>,
    states: &Vec<ItemState>,
    crates: &Crates,
) -> MsgResult<TokenStream> {
    let CodegenIdents {
        events: events_type,
        events_var,
        event_enum,
        state_enum,
        state_var: state,
        exiting_state_var: exiting_state,
        ..
    } = &*CODEGEN_IDENTS;

    let (e_vars, e_variants) =
        (0..events.len()).unzip_vec_into(|i| (event_var(i), event_variant(i)));
    let e_types = events
        .map_vec(|e| crates.get_item_syn_path(cr_idx, &e.path))
        .combine_results();

    let (s_vars, s_variants, s_exit_events) = states.enumer_unzipn_vec(
        |(i, s)| (state_var(i), state_variant(i), event_variant(s.exit_event)),
        Unzip3::unzip3_vec,
    );
    let s_types = states
        .map_vec(|s| crates.get_item_syn_path(cr_idx, &s.path))
        .combine_results();
    let s_data = format_ident!("{STATE_DATA}");
    let s_on_exit = format_ident!("{STATE_EXIT_EVENT}");

    match_ok!(Zip2Msgs, e_types, s_types, {
        quote!(
            struct #events_type {
                #(#e_vars: Vec<#e_types>,)*
                #events_var: std::collections::VecDeque<(#event_enum, usize)>,
                #(#s_vars: Vec<#s_types::#s_data>,)*
                #state: Option<#state_enum>,
                #exiting_state: bool
            }

            impl #events_type {
                fn new() -> Self {
                    Self {
                        #(#e_vars: Vec::new(),)*
                        #events_var: std::collections::VecDeque::new(),
                        #(#s_vars: Vec::new(),)*
                        #state: None,
                        #exiting_state: false
                    }
                }

                fn has_events(&self) -> bool {
                    !self.#events_var.is_empty()
                }

                fn add_event(&mut self, e: #event_enum) {
                    self.#events_var.push_back((e, 0));
                }

                fn get_events(&mut self) -> std::collections::VecDeque<(#event_enum, usize)> {
                    std::mem::replace(&mut self.#events_var, std::collections::VecDeque::new())
                }

                fn append(&mut self, other: &mut Self) {
                    #(
                        other.#e_vars.reverse();
                        self.#e_vars.append(&mut other.#e_vars);
                    )*
                    #(
                        other.#s_vars.reverse();
                        self.#s_vars.append(&mut other.#s_vars);
                    )*
                }

                fn pop(&mut self, e: #event_enum) {
                    match e {
                        #(
                            #event_enum::#e_variants => {
                                self.#e_vars.pop();
                            }
                        )*
                    }
                }

                fn start_state_change(&mut self) -> Option<#event_enum> {
                    self.#exiting_state = true;
                    self.#state.map(|s| match s {
                        #(
                            #state_enum::#s_variants => {
                                self.new_event(#s_types::#s_on_exit);
                                #event_enum::#s_exit_events
                            }
                        )*
                    })
                }

                fn finish_state_change(&mut self, s: S) {
                    self.#exiting_state = false;
                    if let Some(s) = self.#state {
                        match s {
                            #(
                                #state_enum::#s_variants => {
                                    self.#s_vars.pop();
                                }
                            )*
                        }
                    }
                    self.#state = Some(s);
                }
            }
        )
    })
}

impl GetTraitTypes for Vec<ItemEvent> {
    fn get_paths(&self) -> Vec<&ItemPath> {
        self.filter_map_vec(|e| e.state.map_none(&e.path))
    }
}

impl GetTraitTypes for Vec<ItemState> {
    fn get_paths(&self) -> Vec<&ItemPath> {
        self.map_vec(|s| &s.data_path)
    }
}

pub fn event_trait_defs(
    cr_idx: usize,
    events: &Vec<ItemEvent>,
    states: &Vec<ItemState>,
    crates: &Crates,
) -> MsgResult<TokenStream> {
    trait_defs(
        cr_idx,
        crates,
        &CODEGEN_IDENTS.add_event,
        [
            (&ENGINE_TRAITS.add_event, events),
            (&ENGINE_TRAITS.set_state, states),
        ],
    )
}

pub fn event_trait_impls(
    cr_idx: usize,
    events: &Vec<ItemEvent>,
    states: &Vec<ItemState>,
    crates: &Crates,
) -> MsgResult<TokenStream> {
    let macro_cr_idx = crates.get_crate_index(Crate::Macros);

    // Implement trait for every event
    let (e_vars, e_variants) =
        (0..events.len()).unzip_vec_into(|i| (event_var(i), event_variant(i)));
    let e_types = events
        .map_vec(|e| crates.get_item_syn_path(cr_idx, &e.path))
        .combine_results();

    // Implement trait for every state
    let s_vars = (0..states.len()).map_vec_into(|i| state_var(i));
    let s_types = states
        .map_vec(|s| crates.get_item_syn_path(cr_idx, &s.path))
        .combine_results();

    // Implement all dependency traits
    let crate_paths = crates
        .get_crate_syn_paths(cr_idx, [macro_cr_idx])
        .map(|v| v.map_vec_into(|(_, p)| p));

    let CodegenIdents {
        events: events_type,
        event_enum,
        namespace,
        add_event,
        ..
    } = &*CODEGEN_IDENTS;
    let add_event_trait = crates.get_syn_path(cr_idx, &ENGINE_TRAITS.add_event);
    let set_state_trait = crates.get_syn_path(cr_idx, &ENGINE_TRAITS.set_state);

    let data = format_ident!("{STATE_DATA}");
    let on_enter = format_ident!("{STATE_ENTER_EVENT}");

    match_ok!(
        Zip5Msgs,
        e_types,
        s_types,
        crate_paths,
        add_event_trait,
        set_state_trait,
        {
            quote!(
                #(
                    impl #add_event_trait<#e_types> for #events_type {
                        fn new_event(&mut self, t: #e_types) {
                            self.#e_vars.push(t);
                            self.add_event(#event_enum::#e_variants);
                        }

                        fn get_event<'a>(&'a self) -> Option<&'a #e_types> {
                            self.#e_vars.last()
                        }
                    }
                )*
                #(
                    impl #set_state_trait<#s_types::#data> for #events_type {
                        fn set_state(&mut self, t: #s_types::#data) {
                            self.#s_vars.push(t);
                            self.new_event(#s_types::#on_enter)
                        }
                    }
                )*
                #(
                    impl #crate_paths::#namespace::#add_event for #events_type {}
                )*
            )
        }
    )
}
