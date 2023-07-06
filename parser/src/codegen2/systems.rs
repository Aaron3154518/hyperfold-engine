use proc_macro2::TokenStream;
use quote::quote;
use shared::util::{JoinMap, JoinMapInto};

use crate::{
    match_ok,
    resolve::{
        constants::{component_set_var, global_var},
        util::{CombineMsgs, FlattenMsgs, MsgTrait, MsgsResult, Zip2Msgs, Zip3Msgs, Zip6Msgs},
        EnginePaths, EngineTraits, FnArgType, FnArgs, ItemSystem, Items, NamespaceTraits,
    },
};

use super::{
    idents::CodegenIdents,
    util::{vec_to_path, Quote},
    Crates,
};

struct CodegenArgs<'a> {
    cr_idx: usize,
    crates: &'a Crates,
    items: &'a Items,
    func_name: syn::Path,
    args: FnArgs,
    event_trait: &'a syn::Path,
    filter_fn: &'a syn::Path,
    intersect_fn: &'a syn::Path,
    intersect_mut_fn: &'a syn::Path,
}

fn codegen_systems(
    CodegenArgs {
        func_name,
        args,
        event_trait,
        items,
        filter_fn,
        intersect_fn,
        intersect_mut_fn,
        crates,
        cr_idx,
    }: CodegenArgs,
) -> MsgsResult<TokenStream> {
    let globals_var = CodegenIdents::GenGFoo.to_ident();
    let comps_var = CodegenIdents::GenCFoo.to_ident();
    let events_var = CodegenIdents::GenEFoo.to_ident();
    let globals_type = CodegenIdents::GFoo.to_ident();
    let comps_type = CodegenIdents::CFoo.to_ident();
    let events_type = CodegenIdents::EFoo.to_ident();
    let event_arg = CodegenIdents::GenE.to_ident();

    match args {
        FnArgs::Init { mut globals } => {
            globals.sort_by_key(|g| g.arg_idx);
            let func_args = globals.map_vec(|g| {
                let mut_tok = if g.is_mut { quote!(mut) } else { quote!() };
                let var = global_var(g.idx);
                quote!(&#mut_tok self.#globals_var.#var)
            });
            Ok(quote!(fn foo() { let _ = #func_name(#(#func_args),*); }))
        }
        FnArgs::System {
            event,
            globals,
            component_sets,
        } => {
            let num_args = 1 + globals.len() + component_sets.len();
            let mut func_args = (0..num_args).map_vec_into(|_| quote!());
            func_args[event.arg_idx] = event_arg.quote();
            for g in globals {
                func_args[g.arg_idx] = {
                    let mut_tok = if g.is_mut { quote!(mut) } else { quote!() };
                    let var = global_var(g.idx);
                    quote!(&#mut_tok #globals_var.#var)
                };
            }
            let cs = component_sets
                .map_vec(|cs| {
                    let var = component_set_var(cs.idx);
                    func_args[cs.arg_idx] = var.quote();

                    items
                        .component_sets
                        .get(cs.idx)
                        .ok_or(vec![format!("Invalid component set index: {}", cs.idx)])
                        .and_then(|c_set| {
                            crates
                                .get_item_syn_path(cr_idx, &c_set.path)
                                .and_then(|ty| {
                                    c_set.quote(
                                        &ty,
                                        &var,
                                        filter_fn,
                                        intersect_fn,
                                        intersect_mut_fn,
                                    )
                                })
                        })
                        .map(|code| (var, code))
                })
                .combine_msgs();

            let func = quote!(#func_name(#(#func_args),*));

            cs.map(
                |comp_sets| {
                    let (cs_vars, cs_code) = comp_sets.unzip_vec_into(|t|t);
                    let func = match cs_vars.is_empty() {
                        true => quote!(#func),
                        false => quote!(
                            if let (#(Some(#cs_vars),)*) = (#(#cs_vars,)*) {
                                #func
                            }
                        ),
                    };
                    quote!(
                        fn foo() {
                        let _ =|#comps_var: &mut #comps_type, #globals_var: &mut #globals_type, #events_var: &mut #events_type| {
                            if let Some(e) = hyperfold_engine::ecs::events::AddEvent::get_event(efoo) {
                                #(#cs_code)*
                                #func
                            }
                        };
                    }
                    )
                }
            )
        }
    }
}

// TODO: handle erros
pub fn systems(cr_idx: usize, items: &Items, crates: &Crates) -> MsgsResult<Vec<TokenStream>> {
    let event_trait = crates.get_syn_path(cr_idx, EngineTraits::AddEvent);
    let filter_fn = crates.get_syn_path(cr_idx, EnginePaths::Filter);
    let intersect_fn = crates.get_syn_path(cr_idx, EnginePaths::Intersect);
    let intersect_mut_fn = crates.get_syn_path(cr_idx, EnginePaths::IntersectMut);

    items
        .systems
        .map_vec(|system| {
            let event_trait = event_trait.get_ref();
            let filter_fn = filter_fn.get_ref();
            let intersect_fn = intersect_fn.get_ref();
            let intersect_mut_fn = intersect_mut_fn.get_ref();

            let func_name = crates.get_item_syn_path(cr_idx, &system.path);
            let args = system.validate(items);
            match_ok!(
                Zip6Msgs,
                func_name,
                args,
                event_trait,
                filter_fn,
                intersect_fn,
                intersect_mut_fn,
                {
                    codegen_systems(CodegenArgs {
                        func_name,
                        args,
                        event_trait,
                        items,
                        filter_fn,
                        intersect_fn,
                        intersect_mut_fn,
                        crates,
                        cr_idx,
                    })
                }
            )
            .flatten_msgs()
        })
        .filter_vec_into(|r| match r {
            Ok(_) => true,
            Err(err) => {
                eprintln!("{}", err.join("\n"));
                false
            }
        })
        .combine_msgs()
}
