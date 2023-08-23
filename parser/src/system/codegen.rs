use diagnostic::{
    zip_match, CombineResults, ErrForEach, ErrorTrait, FlattenResults, ResultsTrait, ZipResults,
};
use proc_macro2::TokenStream;
use quote::quote;

use shared::{
    syn::{
        error::{CriticalResult, GetVec, MutateResults},
        Quote,
    },
    traits::{CollectVec, CollectVecInto, GetResult},
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

use super::{
    resolve::{EventFnArg, FnArgs, GlobalFnArg},
    ItemSystem,
};

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

pub struct CodegenData<'a> {
    func_name: syn::Path,
    event: EventFnArg,
    globals: Vec<GlobalFnArg>,
    component_sets: Vec<BuildSetsArg<'a>>,
}

pub struct CodegenFuncs<'a> {
    event_trait: &'a syn::Path,
    intersect: &'a syn::Path,
    intersect_opt: &'a syn::Path,
}

pub struct CodegenItems<'a> {
    cr_idx: usize,
    crates: &'a mut Crates,
    items: &'a Items,
    system: &'a ItemSystem,
}

// Codegens event systems
fn codegen_event_system(
    CodegenData {
        func_name,
        event: event_arg,
        globals: global_args,
        component_sets,
    }: CodegenData,
    CodegenFuncs {
        event_trait,
        intersect,
        intersect_opt,
    }: CodegenFuncs,
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

// Splits into init/event systems, calls codegen_event_system for event systems
fn codegen_system(
    func_name: syn::Path,
    args: FnArgs,
    cargs: CodegenItems,
    funcs: CodegenFuncs,
) -> CriticalResult<(TokenStream, Option<syn::Ident>)> {
    let CodegenItems {
        cr_idx,
        crates,
        items,
        system,
    } = cargs;

    match args {
        FnArgs::Init { globals } => Ok((codegen_init_system(globals, func_name), None)),
        FnArgs::System {
            event,
            globals,
            component_sets,
        } => component_sets
            .map_vec_into(|fn_arg| {
                items.component_sets.try_get(fn_arg.idx).and_then(|cs| {
                    crates
                        .get_item_syn_path(cr_idx, &cs.path)
                        .map(|ty| BuildSetsArg { cs, fn_arg, ty })
                })
            })
            .combine_results()
            .with_span(&system.span.span)
            .map(|component_sets| {
                (
                    codegen_event_system(
                        CodegenData {
                            func_name,
                            event,
                            globals,
                            component_sets,
                        },
                        funcs,
                    ),
                    Some(event_variant(event.idx)),
                )
            }),
    }
}

// Validates the system and calls codegen_system
fn validate_system(
    CodegenItems {
        cr_idx,
        crates,
        items,
        system,
    }: CodegenItems,
    funcs: CodegenFuncs,
) -> CriticalResult<(TokenStream, Option<syn::Ident>)> {
    let func_name = crates
        .get_item_syn_path(cr_idx, &system.path)
        .with_span(&system.span.span);
    let args = system.validate(items);
    zip_match!((func_name, args) => {
        codegen_system(
            func_name,
            args,
            CodegenItems {
                cr_idx,
                crates,
                items,
                system
            },
            funcs)
    })
    .flatten_results()
    .map_err(|errs| errs.with_mod(system.span.cr_idx, system.span.m_idx))
}

pub struct SystemsCodegenResult {
    pub init_systems: Vec<TokenStream>,
    pub systems: Vec<TokenStream>,
    pub system_events: Vec<syn::Ident>,
}

pub fn codegen_systems(
    cr_idx: usize,
    items: &Items,
    crates: &mut Crates,
) -> CriticalResult<SystemsCodegenResult> {
    let event_trait = crates.get_syn_path(cr_idx, &ENGINE_TRAITS.add_event);
    let intersect = crates.get_syn_path(cr_idx, &ENGINE_PATHS.intersect);
    let intersect_opt = crates.get_syn_path(cr_idx, &ENGINE_PATHS.intersect_opt);

    let mut init_systems = Vec::new();
    let mut systems = Vec::new();
    let mut system_events = Vec::new();

    zip_match!((event_trait, intersect, intersect_opt) => {
        (&items.systems)
            .try_for_each(|system| {
                validate_system(
                    CodegenItems {
                        cr_idx,
                        crates,
                        items,
                        system
                    },
                    CodegenFuncs {
                        event_trait: &event_trait,
                        intersect: &intersect,
                        intersect_opt: &intersect_opt,
                    }
                )
                .map(|(sys, event)| match event {
                    Some(e) => {
                        system_events.push(e);
                        systems.push(sys);
                    }
                    None => init_systems.push(sys),
                })
            })
            .map(|_| SystemsCodegenResult {
                init_systems,
                systems,
                system_events,
            })
            .critical()
    })
    .flatten_results()
}
