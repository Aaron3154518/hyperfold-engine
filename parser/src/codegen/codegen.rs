use std::{env::temp_dir, fs, path::PathBuf};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use shared::{
    constants::{INDEX, INDEX_SEP},
    match_ok,
    msg_result::{CombineMsgs, MsgResult, MsgTrait, Zip2Msgs, Zip8Msgs, Zip9Msgs},
    traits::{Catch, CollectVec, CollectVecInto, ThenOk},
};

use crate::{
    codegen::Traits,
    resolve::Items,
    utils::{
        constants::NAMESPACE,
        paths::{Crate, NAMESPACE_USE_STMTS},
    },
};

use super::Crates;

pub fn codegen(crates: &Crates, items: &Items, mut errs: Vec<String>) {
    let main_cr_idx = crates.get_crate_index(Crate::Main);
    let macro_cr_idx = crates.get_crate_index(Crate::Macros);

    // Generate globals struct
    let globals = super::globals(main_cr_idx, &items.globals, crates);

    // Generate components struct
    let components = super::components(main_cr_idx, &items.components, crates);

    // Generate component trait implementations
    let component_traits = super::component_trait_impls(main_cr_idx, &items.components, crates);

    // Generate events enum
    let events_enum = super::events_enum(&items.events);

    // Generate events struct
    let events = super::events(main_cr_idx, &items.events, crates);

    // Generate event trait implementations
    let event_traits = super::event_trait_impls(main_cr_idx, &items.events, crates);

    // Generate event/component traits
    let trait_defs = crates
        .iter_except([macro_cr_idx])
        .map_vec_into(|cr| {
            let add_event = super::event_trait_defs(cr.idx, &items.events, crates);
            let add_component = super::component_trait_defs(cr.idx, &items.components, crates);
            match_ok!(Zip2Msgs, add_event, add_component, {
                Traits {
                    add_event,
                    add_component,
                }
            })
        })
        .combine_msgs();

    // Generate manager struct
    let manager_def = super::manager_def();
    let manager_impl = super::manager_impl(main_cr_idx, &items, crates);

    // Generate use statements for namespace
    let use_stmts = crates
        .iter_except([macro_cr_idx])
        .map_vec_into(|cr| {
            NAMESPACE_USE_STMTS
                .map_vec(|path| crates.get_syn_path(cr.idx, path))
                .combine_msgs()
        })
        .combine_msgs()
        .map(|use_stmts| use_stmts.map_vec_into(|stmts| quote!(#(pub use #stmts;)*)));

    // Write codegen to file
    let namespace = format_ident!("{NAMESPACE}");
    let allows = quote!(
        #[allow(unused_imports)]
        #[allow(unused_variables)]
        #[allow(unused_parens)]
        #[allow(dead_code)]
    );
    let errors = MsgResult::new((), errs);
    let code = match_ok!(
        Zip9Msgs,
        errors,
        globals,
        components,
        component_traits,
        events,
        event_traits,
        trait_defs,
        manager_impl,
        use_stmts,
        {
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
                                    #globals
                                    #components
                                    #add_component
                                    #component_traits
                                    #events_enum
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
    );

    let engine_cr_idx = crates.get_crate_index(Crate::Engine);

    match code {
        Ok(code) => write_codegen(crates, code.map_vec_into(|c| c.to_string())),
        Err(errs) => {
            let errs = errs.join("\n");
            let err_msg = "Engine build failed, go to the file below for more information";
            write_codegen(
                crates,
                crates
                    .iter_except([crates.get_crate_index(Crate::Macros)])
                    .map_vec_into(|cr| match cr.idx {
                        i if i == engine_cr_idx => {
                            format!(
                                "compile_error!(\"{err_msg}\");\nconst _: &str = \"\n{errs}\n\";"
                            )
                        }
                        _ => String::new(),
                    }),
            )
        }
    }
}

fn write_codegen(crates: &Crates, code: Vec<String>) {
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("No out directory specified"));

    // Create index file
    fs::write(
        temp_dir().join(INDEX),
        crates
            .iter_except([crates.get_crate_index(Crate::Macros)])
            .zip(code)
            .enumerate()
            .map_vec_into(|(i, (cr, code))| {
                // Write to file
                let file = out.join(format!("{}.rs", i));
                fs::write(file.to_owned(), code)
                    .catch(format!("Could not write to: {}", file.display()));
                format!(
                    "{}{}{}",
                    cr.dir.to_string_lossy().to_string(),
                    INDEX_SEP,
                    file.display()
                )
            })
            .join("\n"),
    )
    .catch(format!("Could not write to index file: {INDEX}"))
}
