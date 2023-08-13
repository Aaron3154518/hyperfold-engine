use diagnostic::{DiagnosticResult, Error};
use proc_macro2::TokenStream;
use quote::quote;

use shared::{
    match_ok,
    msg_result::{CombineMsgs, MsgTrait, ToMsgs, Zip2Msgs, Zip3Msgs, Zip5Msgs},
    syn::Quote,
    traits::{CollectVec, CollectVecInto},
};

use crate::{
    codegen::Crates,
    component_set::{BuildSetsArg, BuildSetsResult, ComponentSet},
    resolve::Items,
    utils::{
        idents::{component_var, event_variant, global_var, CodegenIdents, CODEGEN_IDENTS},
        paths::{ENGINE_PATHS, ENGINE_TRAITS},
    },
};

use super::resolve::{EventFnArg, FnArgs, GlobalFnArg};

fn codegen_init_system(mut global_args: Vec<GlobalFnArg>, func_name: syn::Path) -> TokenStream {
    let CodegenIdents {
        gfoo_var: globals_var,
        ..
    } = &*CODEGEN_IDENTS;
    global_args.sort_by_key(|g| g.arg_idx);
    let func_args = global_args.map_vec(|g| {
        let mut_tok = if g.is_mut { quote!(mut) } else { quote!() };
        let var = global_var(g.idx);
        quote!(&#mut_tok self.#globals_var.#var)
    });
    quote!(#func_name(#(#func_args),*))
}

pub struct CodegenArgs<'a> {
    func_name: syn::Path,
    event_trait: &'a syn::Path,
    intersect: &'a syn::Path,
    intersect_opt: &'a syn::Path,
    event: EventFnArg,
    globals: Vec<GlobalFnArg>,
    component_sets: Vec<BuildSetsArg<'a>>,
}

fn codegen_system(
    CodegenArgs {
        func_name,
        event_trait,
        intersect,
        intersect_opt,
        event: event_arg,
        globals: global_args,
        component_sets,
    }: CodegenArgs,
) -> TokenStream {
    let CodegenIdents {
        globals,
        components,
        events,
        e_var,
        cfoo_var: comps_var,
        gfoo_var: globals_var,
        efoo_var: events_var,
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
    } = ComponentSet::codegen_build_sets(&component_sets, intersect, intersect_opt);
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
        if let Some(#e_var) = #event_trait::get_event(#events_var) {
            #build_sets_code
            #func
        }
    )
}

pub struct SystemsCodegenResult {
    pub init_systems: Vec<TokenStream>,
    pub systems: Vec<TokenStream>,
    pub system_events: Vec<syn::Ident>,
}

pub fn codegen_systems(
    cr_idx: usize,
    items: &Items,
    crates: &Crates,
) -> DiagnosticResult<SystemsCodegenResult> {
    let event_trait = crates.get_syn_path(cr_idx, &ENGINE_TRAITS.add_event);
    let intersect = crates.get_syn_path(cr_idx, &ENGINE_PATHS.intersect);
    let intersect_opt = crates.get_syn_path(cr_idx, &ENGINE_PATHS.intersect_opt);

    let mut init_systems = Vec::new();
    let mut systems = Vec::new();
    let mut system_events = Vec::new();

    let mut errs = Vec::new();

    match_ok!(Zip3Msgs, event_trait, intersect, intersect_opt, {
        for system in &items.systems {
            let func_name = crates.get_item_syn_path(cr_idx, &system.path);
            let args = system.validate(items);
            match_ok!(Zip2Msgs, func_name, args, {
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
                            .map_vec_into(|fn_arg| {
                                items
                                    .component_sets
                                    .get(fn_arg.idx)
                                    .ok_or(vec![Error::new(&format!(
                                        "Invalid component set index: {}",
                                        fn_arg.idx
                                    ))])
                                    .and_then(|cs| {
                                        crates
                                            .get_item_syn_path(cr_idx, &cs.path)
                                            .map(|ty| BuildSetsArg { cs, fn_arg, ty })
                                    })
                            })
                            .combine_results();

                        system_events.push(event_variant(event.idx));

                        systems.push(component_sets.map(|component_sets| {
                            codegen_system(CodegenArgs {
                                func_name,
                                event_trait: &event_trait,
                                intersect: &intersect,
                                intersect_opt: &intersect_opt,
                                event,
                                globals,
                                component_sets,
                            })
                        }));
                    }
                }
            })
            .record_errs(&mut errs);
        }
    });

    systems
        .combine_results()
        .take_errs(errs.err_or(()))
        .map(|systems| SystemsCodegenResult {
            init_systems,
            systems,
            system_events,
        })
}
