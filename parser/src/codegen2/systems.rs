use proc_macro2::TokenStream;
use quote::quote;
use shared::util::{JoinMap, JoinMapInto};

use crate::{
    match_ok,
    resolve::{
        constants::{component_set_keys_fn, component_set_var, global_var},
        util::{
            CombineMsgs, FlattenMsgs, MsgTrait, MsgsResult, Zip2Msgs, Zip3Msgs, Zip4Msgs, Zip6Msgs,
        },
        ComponentSet, ComponentSetFnArg, EnginePaths, EngineTraits, EventFnArg, FnArgType, FnArgs,
        GlobalFnArg, ItemSystem, Items, NamespaceTraits,
    },
};

use super::{
    idents::CodegenIdents,
    util::{vec_to_path, Quote},
    Crates,
};

fn codegen_init_system(mut globals: Vec<GlobalFnArg>, func_name: syn::Path) -> TokenStream {
    let globals_var = CodegenIdents::GFooVar.to_ident();
    globals.sort_by_key(|g| g.arg_idx);
    let func_args = globals.map_vec(|g| {
        let mut_tok = if g.is_mut { quote!(mut) } else { quote!() };
        let var = global_var(g.idx);
        quote!(&#mut_tok self.#globals_var.#var)
    });
    quote!(fn foo() { let _ = #func_name(#(#func_args),*); })
}

pub struct CodegenSystemArgs<'a> {
    func_name: syn::Path,
    event_trait: &'a syn::Path,
    intersect: &'a syn::Path,
    event: EventFnArg,
    globals: Vec<GlobalFnArg>,
    component_sets: Vec<(ComponentSetFnArg, &'a ComponentSet, syn::Path)>,
}

fn codegen_system(
    CodegenSystemArgs {
        func_name,
        event_trait,
        intersect,
        event,
        globals,
        component_sets,
    }: CodegenSystemArgs,
) -> TokenStream {
    let globals_var = CodegenIdents::GFooVar.to_ident();
    let comps_var = CodegenIdents::CFooVar.to_ident();
    let events_var = CodegenIdents::EFooVar.to_ident();
    let globals_type = CodegenIdents::GFooType.to_ident();
    let comps_type = CodegenIdents::CFooType.to_ident();
    let events_type = CodegenIdents::EFooType.to_ident();
    let event_arg = CodegenIdents::GenE.to_ident();

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
    let (build_cs, cs_args, cs_singletons) = ComponentSet::quote_args(&component_sets, intersect);
    for ((arg, ..), tok) in component_sets.iter().zip(cs_args) {
        func_args[arg.arg_idx] = tok;
    }

    let func = quote!(#func_name(#(#func_args),*));

    // Singleton options
    let func = match cs_singletons.is_empty() {
        true => quote!(#func),
        false => quote!(
            if let (#(Some(#cs_singletons),)*) = (#(#cs_singletons,)*) {
                #func
            }
        ),
    };

    quote!(
        fn foo() {
            let _ =|#comps_var: &mut #comps_type, #globals_var: &mut #globals_type, #events_var: &mut #events_type| {
                if let Some(e) = #event_trait::get_event(efoo) {
                    #build_cs
                    #func
                }
            };
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
) -> MsgsResult<TokenStream> {
    match args {
        FnArgs::Init { globals } => Ok(codegen_init_system(globals, func_name)),
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

// TODO: handle erros
pub fn systems(cr_idx: usize, items: &Items, crates: &Crates) -> MsgsResult<Vec<TokenStream>> {
    let event_trait = crates.get_syn_path(cr_idx, EngineTraits::AddEvent);
    let intersect = crates.get_syn_path(cr_idx, EnginePaths::Intersect);

    items
        .systems
        .map_vec(|system| {
            let event_trait = event_trait.get_ref();
            let intersect = intersect.get_ref();

            let func_name = crates.get_item_syn_path(cr_idx, &system.path);
            let args = system.validate(items);
            match_ok!(Zip4Msgs, func_name, args, event_trait, intersect, {
                codegen_systems(CodegenArgs {
                    cr_idx,
                    crates,
                    items,
                    args,
                    func_name,
                    event_trait,
                    intersect,
                })
            })
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
