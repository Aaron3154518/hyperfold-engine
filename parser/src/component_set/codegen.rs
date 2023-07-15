use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use shared::{
    match_ok,
    msg_result::{MsgResult, Zip3Msgs},
    traits::{CollectVec, CollectVecInto},
};

use crate::{
    codegen::Crates,
    parse::ComponentSymbol,
    system::ComponentSetFnArg,
    utils::{
        idents::{
            component_set_keys_fn, component_set_var, component_var, CodegenIdents, CODEGEN_IDENTS,
        },
        paths::ENGINE_PATHS,
        syn::{get_fn_name, Quote},
    },
};

use super::{
    labels::ComponentSetLabels,
    resolve::{ComponentSetItem, LabelItem},
    ComponentSet,
};

impl LabelItem {
    fn quote<F>(&self, f: &F) -> TokenStream
    where
        F: Fn(ComponentSymbol) -> syn::Ident,
    {
        match self {
            LabelItem::Item { not, comp } => {
                let not = if *not { quote!(!) } else { quote!() };
                let var = f(*comp);
                quote!(#not #var)
            }
            LabelItem::Expression { op, items } => {
                let vars = items
                    .iter()
                    .map(|i| i.quote(f))
                    .intersperse(op.quote())
                    .collect::<Vec<_>>();
                quote!((#(#vars)*))
            }
        }
    }
}

pub struct BuildSetsResult {
    pub build_sets_code: TokenStream,
    pub func_args: Vec<TokenStream>,
    pub singletons: Vec<TokenStream>,
}

pub struct BuildSetsArg<'a> {
    pub cs: &'a ComponentSet,
    pub fn_arg: ComponentSetFnArg,
    pub ty: syn::Path,
}

impl ComponentSet {
    pub fn codegen_get_keys_fns(
        cr_idx: usize,
        component_sets: &Vec<Self>,
        crates: &Crates,
    ) -> MsgResult<Vec<TokenStream>> {
        let filter_fn = crates.get_syn_path(cr_idx, &ENGINE_PATHS.filter);
        let entity_set = crates.get_syn_path(cr_idx, &ENGINE_PATHS.entity_set);
        let entity = crates.get_syn_path(cr_idx, &ENGINE_PATHS.entity);

        match_ok!(Zip3Msgs, filter_fn, entity, entity_set, {
            component_sets.enumerate_map_vec(|(i, cs)| {
                cs.codegen_get_keys(i, &filter_fn, &entity, &entity_set)
            })
        })
    }

    // Generates function which get the keys used in a component set
    fn codegen_get_keys(
        &self,
        cs_idx: usize,
        filter: &syn::Path,
        entity: &syn::Path,
        entity_set: &syn::Path,
    ) -> TokenStream {
        // Remove duplicate arguments
        let mut args = self.args.to_vec();
        args.sort_by_key(|item| item.comp.idx);
        args.dedup_by_key(|item| item.comp.idx);

        // Get first arg, priority to singleton
        let singleton_arg = args.iter().find(|item| item.comp.args.is_singleton);
        let first_arg = singleton_arg.or(args.first());

        // Generate final function signatures in case we return early
        let CodegenIdents {
            components,
            eids_var,
            comps_var,
            ..
        } = &*CODEGEN_IDENTS;

        let get_keys = component_set_keys_fn(cs_idx, false);
        let get_keys_fn = quote!(#get_keys<'a>(#comps_var: &#components, #eids_var: &'a #entity_set) -> Option<&'a #entity>);
        let get_keys_vec = component_set_keys_fn(cs_idx, true);
        let get_keys_vec_fn = quote!(#get_keys_vec<'a>(#comps_var: &#components, #eids_var: &'a #entity_set) -> Vec<(&'a #entity, ())>);

        // Get labels expression and first/singleton label
        let mut first = self.first_arg_label();
        let labels = match &self.labels {
            Some(labels) => match labels {
                // Label expression is always true, ignore labels
                ComponentSetLabels::Constant(true) => None,
                // Label expression is always false, skip computations
                ComponentSetLabels::Constant(false) => {
                    let fn_vec = quote!(fn #get_keys_vec_fn { vec![] });
                    return match singleton_arg {
                        Some(_) => quote!(
                            fn #get_keys_fn { None }
                            #fn_vec
                        ),
                        None => fn_vec,
                    };
                }
                ComponentSetLabels::Expression(expr) => Some(expr),
            },
            None => None,
        };

        // Create statement to initialize the result
        // Returns
        //   Has Singleton ? Option<&'a K> : Vec<(&'a K, ())>
        //   Where 'a is bound to #eids
        let init = match first {
            // Arg or label
            Some(comp) if comp.args.is_singleton => {
                let var = component_var(comp.idx);
                // Option<K>
                quote!(#comps_var.#var.get_key().and_then(|k| #eids_var.get(k)))
            }
            Some(comp) => {
                let var = component_var(comp.idx);
                // Vec<(K, V)>
                quote!(#comps_var.#var.keys().filter_map(|k| #eids_var.get(k).map(|k| (k, ()))).collect::<Vec<_>>())
            }
            // No arg and no label
            None => {
                // Vec<(K, V)>
                quote!(#eids_var.iter().map(|k| (k, ())).collect::<Vec<_>>())
            }
        };

        // Add filter if labels are present
        let cs = component_set_var(cs_idx);
        let init_and_filter = match &labels {
            Some(expr) => {
                let f = quote!(f);
                let label_expr = expr.labels.quote(&|comp| component_var(comp.idx));
                let label_vars = expr
                    .iter_symbols()
                    .map_vec_into(|comp| component_var(comp.idx));
                let num_labels = label_vars.len();
                let label_fn = quote!(|[#(#label_vars),*]: [bool; #num_labels]| #label_expr);
                let eval_labels = quote!(#f([#(#comps_var.#label_vars.contains_key(k)),*]));
                match first {
                    // Option<K>
                    Some(comp) if comp.args.is_singleton => {
                        quote!(
                            let #cs = #init;
                            let #f = #label_fn;
                            let #cs = #cs.and_then(|k| #eval_labels.then_some(k));
                        )
                    }
                    // Vec<(K, V)>
                    _ => quote!(
                        let mut #cs = #init;
                        let #f = #label_fn;
                        #cs.drain_filter(|(k, _)| !#eval_labels);
                    ),
                }
            }
            None => quote!(let #cs = #init;),
        };

        // Generate final functions
        match first {
            // Option<K>
            Some(comp) if comp.args.is_singleton => {
                quote!(
                    fn #get_keys_fn {
                        #init_and_filter
                        #cs
                    }
                    fn #get_keys_vec_fn {
                        Self::#get_keys(#comps_var, #eids_var).map_or(vec![], |t| vec![(t, ())])
                    }
                )
            }
            // Vec<K>
            _ => {
                quote!(
                    fn #get_keys_vec_fn {
                        #init_and_filter
                        #cs
                    }
                )
            }
        }
    }

    // Generates inline code to construct values from keys
    fn codegen_get_vals(
        &self,
        v: syn::Ident,
        ty: &syn::Path,
        intersect: &syn::Path,
    ) -> TokenStream {
        // Remove duplicate arguments
        let mut args = self.args.to_vec();
        args.sort_by_key(|item| item.comp.idx);
        args.dedup_by_key(|item| item.comp.idx);

        let (first_arg, first_label) = (self.first_arg(), self.first_label());

        let CodegenIdents {
            eid_var, comps_var, ..
        } = &*CODEGEN_IDENTS;

        // Unique component variables
        let var = args.map_vec(|item| component_var(item.comp.idx));
        // Full list of component names and variables
        let arg_name = self.args.map_vec(|item| format_ident!("{}", item.var));
        let arg_var = self.args.map_vec(|item| component_var(item.comp.idx));
        match (first_arg, first_label) {
            // Option<K>, No args
            (None, Some(comp)) if comp.args.is_singleton => {
                quote!(let #v = #v.map(|k| #ty { #eid_var: k });)
            }
            // Option<K>
            (Some(ComponentSetItem { comp, .. }), _) | (_, Some(comp))
                if comp.args.is_singleton =>
            {
                let get = args.map_vec(|item| get_fn_name("get", item.is_mut));
                quote!(
                    let #v = #v.and_then(|k| match (#(#comps_var.#var.#get(k)),*) {
                        (#(Some(#var)),*) => Some(#ty {
                            #eid_var: k,
                            #(#arg_name: #arg_var),*
                        }),
                        _ => None
                    });
                )
            }
            // Vec<(K, V)>, No args
            (None, _) => {
                quote!(
                    let #v = #v.into_iter()
                        .map(|(k, _)| #ty { eid: k })
                        .collect();
                )
            }
            // Vec<(K, V)>
            _ => {
                let value_fn = (0..args.len()).map_vec_into(|i| match i {
                    0 => quote!(|_, v| v),
                    i => {
                        let tmps = (0..i).map_vec_into(|i| format_ident!("v{i}"));
                        let tmp_n = format_ident!("v{}", i);
                        quote!(|(#(#tmps),*), #tmp_n| (#(#tmps,)* #tmp_n))
                    }
                });
                let iter = args.map_vec(|item| get_fn_name("iter", item.is_mut));
                quote!(
                    #(let #v = #intersect(#v, #comps_var.#var.#iter(), #value_fn);)*
                    let #v = #v.into_iter()
                        .map(|(k, (#(#var),*))| #ty {
                            #eid_var: k,
                            #(#arg_name: #arg_var),*
                        })
                        .collect();
                )
            }
        }
    }

    fn codegen_copy(other: syn::Ident, ty: &syn::Path, vars: Vec<syn::Ident>) -> TokenStream {
        let eid = &CODEGEN_IDENTS.eid_var;
        let c = quote!(
            #ty {
                #eid: #other.#eid,
                #(#vars: #other.#vars),*
            }
        );
        eprintln!("{c}");
        c
    }

    fn codegen_copy_vec(other: syn::Ident, ty: &syn::Path, vars: Vec<syn::Ident>) -> TokenStream {
        let v = format_ident!("v");
        let copy = Self::codegen_copy(v.clone(), ty, vars);
        let c = quote!(#other.iter().map(|#v| #copy).collect());
        eprintln!("{c}");
        c
    }

    // Param 1: Vec<(cs function arg, cs, cs type)>
    pub fn codegen_build_sets(
        component_sets: &Vec<BuildSetsArg>,
        intersect: &syn::Path,
    ) -> BuildSetsResult {
        let CodegenIdents {
            eids_var,
            comps_var,
            ..
        } = &*CODEGEN_IDENTS;

        let mut c_sets = component_sets.iter().collect::<Vec<_>>();
        c_sets.sort_by_key(|cs| cs.fn_arg.idx);
        c_sets.dedup_by_key(|cs| cs.fn_arg.idx);

        // Get all the keys
        // Now we have immutable references to eids
        let (var, get_keys_fn) = c_sets.unzip_vec(|cs| {
            (
                component_set_var(cs.fn_arg.idx),
                component_set_keys_fn(cs.fn_arg.idx, cs.fn_arg.is_vec),
            )
        });

        // Build sets
        let get_vals = c_sets
            .iter()
            .zip(var.iter())
            .map_vec_into(|(cs, v)| cs.cs.codegen_get_vals(v.clone(), &cs.ty, intersect));

        BuildSetsResult {
            build_sets_code: quote!(
                #(let #var = Self::#get_keys_fn(#comps_var, &#comps_var.#eids_var);)*
                #(#get_vals)*
            ),
            func_args: component_sets.map_vec(|cs| {
                let var = component_set_var(cs.fn_arg.idx);
                match component_sets
                    .iter()
                    .rev()
                    .find(|cs2| cs.fn_arg.idx == cs2.fn_arg.idx)
                {
                    Some(cs2) if cs2.fn_arg.arg_idx != cs.fn_arg.arg_idx => {
                        match cs.fn_arg.is_vec {
                            true => Self::codegen_copy_vec(
                                var,
                                &cs.ty,
                                cs.cs.args.map_vec(|arg| format_ident!("{}", arg.var)),
                            ),
                            false => Self::codegen_copy(
                                var,
                                &cs.ty,
                                cs.cs.args.map_vec(|arg| format_ident!("{}", arg.var)),
                            ),
                        }
                    }
                    _ => var.quote(),
                }
            }),
            singletons: c_sets.filter_map_vec(|cs| match cs.cs.has_singleton() {
                true => Some(component_set_var(cs.fn_arg.idx).quote()),
                false => None,
            }),
        }
    }
}
