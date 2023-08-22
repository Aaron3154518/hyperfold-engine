use diagnostic::{zip_match, CatchErr, CombineResults, ZipResults};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::{env::temp_dir, fs, path::PathBuf};

use shared::{
    constants::{INDEX, INDEX_SEP},
    syn::{
        error::{Result, StrToError},
        ToRange,
    },
    traits::{Catch, CollectVec, CollectVecInto, ThenOk},
};

use crate::{
    codegen::Traits,
    parse::AstCrate,
    resolve::Items,
    utils::{
        constants::NAMESPACE,
        paths::{Crate, MAIN_USE_STMTS, NAMESPACE_USE_STMTS},
    },
};

use super::Crates;

pub fn codegen(crates: &mut Crates, items: &Items) -> Result<Vec<TokenStream>> {
    let main_cr_idx = crates.get_crate_index(Crate::Main);
    let macro_cr_idx = crates.get_crate_index(Crate::Macros);

    // Generate globals struct
    let globals = super::globals(main_cr_idx, &items.globals, crates);

    // Generate components struct
    let components = super::components(main_cr_idx, &items.components, crates);

    // Generate component trait implementations
    let component_traits = super::component_trait_impls(main_cr_idx, &items.components, crates);

    // Generate events/states enums
    let events_enums = super::events_enums(&items.events, &items.states);

    // Generate events struct
    let events = super::events(main_cr_idx, &items.events, &items.states, crates);

    // Generate event trait implementations
    let event_traits = super::event_trait_impls(main_cr_idx, &items.events, &items.states, crates);

    // Generate event/component traits
    let trait_defs = crates
        .iter_except([macro_cr_idx])
        .map_vec_into(|cr| {
            let add_event = super::event_trait_defs(cr.idx, &items.events, &items.states, crates);
            let add_component = super::component_trait_defs(cr.idx, &items.components, crates);
            zip_match!((add_event, add_component) => {
                Traits {
                    add_event,
                    add_component,
                }
            })
        })
        .combine_results();

    // Generate manager struct
    let manager_def = super::manager_def();
    let manager_impl = super::manager_impl(main_cr_idx, &items, crates);

    // Generate use statements for namespace
    let use_stmts = crates
        .iter_except([macro_cr_idx])
        .map_vec_into(|cr| {
            NAMESPACE_USE_STMTS
                .map_vec(|path| crates.get_syn_path(cr.idx, path))
                .combine_results()
        })
        .combine_results()
        .map(|use_stmts| use_stmts.map_vec_into(|stmts| quote!(#(pub use #stmts;)*)));

    let main_use_stmts = MAIN_USE_STMTS
        .map_vec(|path| crates.get_syn_path(main_cr_idx, path))
        .combine_results()
        .map(|use_stmts| quote!(#(use #use_stmts;)*));

    // Write codegen to file
    let namespace = format_ident!("{NAMESPACE}");
    let allows = quote!(
        #[allow(unused_imports)]
        #[allow(unused_variables)]
        #[allow(unused_parens)]
        #[allow(dead_code)]
    );
    zip_match!(
        (
            globals, components, component_traits,
            events, event_traits, trait_defs,
            manager_impl, use_stmts, main_use_stmts
        ) => {
            trait_defs
                .into_iter()
                .zip(use_stmts)
                .enumerate()
                .map_vec_into(
                    |(
                        cr_idx,
                        (
                            Traits {
                                add_event,
                                add_component,
                            },
                            use_stmts,
                        ),
                    )| {
                        if cr_idx == main_cr_idx {
                            quote!(
                                #allows
                                pub mod #namespace {
                                    #use_stmts
                                    #main_use_stmts
                                    #globals
                                    #components
                                    #add_component
                                    #component_traits
                                    #events_enums
                                    #events
                                    #add_event
                                    #event_traits
                                    #manager_def
                                    #manager_impl
                                }
                            )
                        } else {
                            quote!(
                                #allows
                                pub mod #namespace {
                                    #use_stmts
                                    #add_component
                                    #add_event
                                }
                            )
                        }
                    },
                )
        }
    )
}

pub fn write_codegen(code: Vec<(&AstCrate, String)>) -> Result<()> {
    let out = PathBuf::from(std::env::var("OUT_DIR").catch_err("No out dir specified".trace())?);

    let mut index_lines = Vec::new();
    for (i, (cr, code)) in code.into_iter().enumerate() {
        // Write to file
        let file = out.join(format!("{}.rs", i));
        fs::write(file.to_owned(), code)
            .catch_err(format!("Could not write to: {}", file.display()).trace())?;
        index_lines.push(format!(
            "{}{}{}",
            cr.dir.to_string_lossy().to_string(),
            INDEX_SEP,
            file.display()
        ));
    }

    // Create index file
    fs::write(temp_dir().join(INDEX), index_lines.join("\n"))
        .catch_err(format!("Could not write to index file: {INDEX}").trace())
}
