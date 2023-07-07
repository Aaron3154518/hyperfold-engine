use proc_macro2::TokenStream;
use quote::quote;
use shared::util::{JoinMap, JoinMapInto};

use crate::{
    match_ok,
    resolve::{
        constants::{component_set_fn, component_set_var, global_var},
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
}

fn codegen_systems(
    CodegenArgs {
        func_name,
        args,
        event_trait,
        items,
        crates,
        cr_idx,
    }: CodegenArgs,
) -> TokenStream {
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
            quote!(fn foo() { let _ = #func_name(#(#func_args),*); })
        }
        FnArgs::System {
            event,
            globals,
            mut component_sets,
        } => {
            // Generate function argument tokens
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
            for (i, cs) in component_sets.get_enumerate() {
                let var = component_set_var(cs.idx);
                func_args[cs.arg_idx] =
                    match component_sets.iter().rev().find(|cs2| cs2.idx == cs.idx) {
                        Some(cs2) if cs2.arg_idx != cs.arg_idx => quote!(#var.clone()),
                        _ => var.quote(),
                    };
            }
            // Generate component set builder code
            component_sets.sort_by_key(|cs| cs.idx);
            component_sets.dedup_by_key(|cs| cs.idx);
            let (cs_vars, cs_fns) = component_sets
                .unzip_vec(|cs| (component_set_var(cs.idx), component_set_fn(cs.idx)));

            let func = quote!(#func_name(#(#func_args),*));

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
                            #(#cs_vars = Self::#cs_fns(#comps_var);)*
                            #func
                        }
                    };
                }
            )
        }
    }
}

// TODO: handle erros
pub fn systems(cr_idx: usize, items: &Items, crates: &Crates) -> MsgsResult<Vec<TokenStream>> {
    let event_trait = crates.get_syn_path(cr_idx, EngineTraits::AddEvent);

    items
        .systems
        .map_vec(|system| {
            let event_trait = event_trait.get_ref();

            let func_name = crates.get_item_syn_path(cr_idx, &system.path);
            let args = system.validate(items);
            match_ok!(Zip3Msgs, func_name, args, event_trait, {
                codegen_systems(CodegenArgs {
                    func_name,
                    args,
                    event_trait,
                    items,
                    crates,
                    cr_idx,
                })
            })
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
