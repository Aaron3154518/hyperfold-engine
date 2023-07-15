use proc_macro2::TokenStream;
use quote::quote;
use shared::{
    match_ok,
    msg_result::{MsgResult, Zip4Msgs},
    traits::{CollectVec, CollectVecInto},
};

use crate::{
    codegen::Crates,
    component_set::{BuildSetsArg, BuildSetsResult, ComponentSet},
    resolve::{Items, ENGINE_PATHS, ENGINE_TRAITS},
    utils::{
        idents::{event_variant, global_var, CodegenIdents, CODEGEN_IDENTS},
        syn::Quote,
    },
};

use super::resolve::{EventFnArg, FnArgs, GlobalFnArg};

fn codegen_init_system(mut global_args: Vec<GlobalFnArg>, func_name: syn::Path) -> TokenStream {
    let CodegenIdents { globals_var, .. } = &*CODEGEN_IDENTS;
    global_args.sort_by_key(|g| g.arg_idx);
    let func_args = global_args.map_vec(|g| {
        let mut_tok = if g.is_mut { quote!(mut) } else { quote!() };
        let var = global_var(g.idx);
        quote!(&#mut_tok self.#globals_var.#var)
    });
    quote!(#func_name(#(#func_args),*))
}

pub struct CodegenSystemArgs<'a> {
    func_name: syn::Path,
    event_trait: &'a syn::Path,
    intersect: &'a syn::Path,
    event: EventFnArg,
    globals: Vec<GlobalFnArg>,
    component_sets: Vec<BuildSetsArg<'a>>,
}

fn codegen_system(
    CodegenSystemArgs {
        func_name,
        event_trait,
        intersect,
        event: event_arg,
        globals: global_args,
        component_sets,
    }: CodegenSystemArgs,
) -> TokenStream {
    let CodegenIdents {
        globals,
        components,
        events,
        e_var,
        comps_var,
        globals_var,
        events_var,
        ..
    } = &*CODEGEN_IDENTS;

    // Generate function argument tokens
    let num_args = 1 + global_args.len() + component_sets.len();
    let mut func_args = (0..num_args).map_vec_into(|_| quote!());
    func_args[event_arg.arg_idx] = e_var.quote();
    for g in global_args {
        func_args[g.arg_idx] = {
            let mut_tok = if g.is_mut { quote!(mut) } else { quote!() };
            let var = global_var(g.idx);
            quote!(&#mut_tok #globals_var.#var)
        };
    }
    let BuildSetsResult {
        build_sets_code,
        func_args: cs_func_args,
        singletons,
    } = ComponentSet::codegen_build_sets(&component_sets, intersect);
    for (cs, tok) in component_sets.iter().zip(cs_func_args) {
        func_args[cs.fn_arg.arg_idx] = tok;
    }

    let func = quote!(#func_name(#(#func_args),*));

    // Singleton options
    let func = match singletons.is_empty() {
        true => quote!(#func),
        false => quote!(
            if let (#(Some(#singletons),)*) = (#(#singletons,)*) {
                #func
            }
        ),
    };

    quote!(
        |#comps_var: &mut #components, #globals_var: &mut #globals, #events_var: &mut #events| {
            if let Some(#e_var) = #event_trait::get_event(#events_var) {
                #build_sets_code
                #func
            }
        }
    )
}

struct CodegenArgs<'a> {
    cr_idx: usize,
    crates: &'a Crates,
    items: &'a Items,
    args: FnArgs,
    func_name: syn::Path,
    event_trait: &'a syn::Path,
    intersect: &'a syn::Path,
}

fn codegen_systems(
    CodegenArgs {
        cr_idx,
        crates,
        items,
        args,
        func_name,
        event_trait,
        intersect,
    }: CodegenArgs,
) -> MsgResult<TokenStream> {
    match args {
        FnArgs::Init { globals } => Ok(codegen_init_system(globals, func_name)),
        FnArgs::System {
            event,
            globals,
            component_sets,
        } => {
            let component_sets = component_sets
                .map_vec_into(|fn_arg| {
                    items
                        .component_sets
                        .get(fn_arg.idx)
                        .ok_or(vec![format!("Invalid component set index: {}", fn_arg.idx)])
                        .and_then(|cs| {
                            crates
                                .get_item_syn_path(cr_idx, &cs.path)
                                .map(|ty| BuildSetsArg { cs, fn_arg, ty })
                        })
                })
                .combine_msgs();

            component_sets.map(|component_sets| {
                codegen_system(CodegenSystemArgs {
                    func_name,
                    event_trait,
                    intersect,
                    event,
                    globals,
                    component_sets,
                })
            })
        }
    }
}

pub struct SystemsCodegenResult {
    pub init_systems: Vec<TokenStream>,
    pub systems: Vec<TokenStream>,
    pub system_events: Vec<syn::Ident>,
}

// TODO: handle erros
pub fn systems(cr_idx: usize, items: &Items, crates: &Crates) -> MsgResult<SystemsCodegenResult> {
    let event_trait = crates.get_syn_path(cr_idx, &ENGINE_TRAITS.add_event);
    let intersect = crates.get_syn_path(cr_idx, &ENGINE_PATHS.intersect);

    let mut init_systems = Vec::new();
    let mut systems = Vec::new();
    let mut system_events = Vec::new();

    for system in &items.systems {
        let event_trait = event_trait.get_ref();
        let intersect = intersect.get_ref();

        let func_name = crates.get_item_syn_path(cr_idx, &system.path);
        let args = system.validate(items);
        match_ok!(Zip4Msgs, func_name, args, event_trait, intersect, {
            match args {
                FnArgs::Init { globals } => {
                    init_systems.push(codegen_init_system(globals, func_name))
                }
                FnArgs::System {
                    event,
                    globals,
                    component_sets,
                } => {
                    let component_sets = component_sets
                        .map_vec_into(|arg| {
                            items
                                .component_sets
                                .get(arg.idx)
                                .ok_or(vec![format!("Invalid component set index: {}", arg.idx)])
                                .and_then(|cs| {
                                    crates
                                        .get_item_syn_path(cr_idx, &cs.path)
                                        .map(|p| (arg, cs, p))
                                })
                        })
                        .combine_msgs();

                    system_events.push(event_variant(event.idx));

                    systems.push(component_sets.map(|component_sets| {
                        codegen_system(CodegenSystemArgs {
                            func_name,
                            event_trait,
                            intersect,
                            event,
                            globals,
                            component_sets,
                        })
                    }));
                }
            }
        });
    }

    systems
        .filter_vec_into(|r| match r {
            Ok(_) => true,
            Err(err) => {
                eprintln!("{}", err.join("\n"));
                false
            }
        })
        .combine_msgs()
        .map(|systems| SystemsCodegenResult {
            init_systems,
            systems,
            system_events,
        })
}
