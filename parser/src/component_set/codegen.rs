use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use shared::{
    match_ok,
    msg_result::Zip3Msgs,
    syn::{get_fn_name, MsgResult, Quote},
    traits::{CollectVec, CollectVecInto, ThenNone},
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
            component_sets
                .enumer_map_vec(|(i, cs)| cs.codegen_get_keys(i, &filter_fn, &entity, &entity_set))
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
            cfoo_var: comps_var,
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
                        #cs.extract_if(|(k, _)| !#eval_labels).count();
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
        arg: &ComponentSetFnArg,
        v: syn::Ident,
        ty: &syn::Path,
        intersect: &syn::Path,
        intersect_opt: &syn::Path,
    ) -> TokenStream {
        // Remove duplicate arguments
        let mut args = self.args.to_vec();
        args.sort_by_key(|item| item.comp.idx);
        args.dedup_by_key(|item| item.comp.idx);

        let (first_arg, first_label) = (self.first_arg(), self.first_label());

        let CodegenIdents {
            eid_var,
            cfoo_var: comps_var,
            ..
        } = &*CODEGEN_IDENTS;

        let k = format_ident!("k");

        match (first_arg, first_label) {
            // Option<K>
            (Some(ComponentSetItem { comp, .. }), _) | (_, Some(comp))
                if comp.args.is_singleton && !arg.is_vec =>
            {
                // Unique args
                let var =
                    args.filter_map_vec(|item| item.is_opt.then_none(component_var(item.comp.idx)));

                let (mut arg_name, mut arg_var) = (Vec::new(), Vec::new());
                let (mut opt_arg_name, mut opt_arg_var, mut opt_arg_get) =
                    (Vec::new(), Vec::new(), Vec::new());
                for item in &self.args {
                    let (names, vars) = match item.is_opt {
                        true => {
                            opt_arg_get.push(item.get_fn());
                            (&mut opt_arg_name, &mut opt_arg_var)
                        }
                        false => (&mut arg_name, &mut arg_var),
                    };
                    names.push(format_ident!("{}", item.var));
                    vars.push(component_var(item.comp.idx));
                }

                // Expression to assign vars
                let new = quote!(
                    #ty {
                        #eid_var: #k
                        #(,#arg_name: #arg_var)*
                        #(,#opt_arg_name: #comps_var.#opt_arg_var.#opt_arg_get(#k))*
                    }
                );

                match first_arg {
                    Some(_) => {
                        let var_get =
                            args.filter_map_vec(|item| item.is_opt.then_none(item.get_fn()));
                        quote!(
                            let #v = #v.and_then(|#k| match (#(#comps_var.#var.#var_get(#k)),*) {
                                (#(Some(#var)),*) => Some(#new),
                                _ => None
                            });
                        )
                    }
                    // No args
                    None => quote!(let #v = #v.map(|#k| #new);),
                }
            }
            // Vec<(K, V)>
            _ => {
                // Unique args
                let (mut var, mut var_intersect) = (Vec::new(), Vec::new());
                for item in &args {
                    var.push(component_var(item.comp.idx));
                    var_intersect.push(match item.is_opt {
                        true => intersect_opt,
                        false => intersect,
                    });
                }

                let (mut arg_name, mut arg_var) = (Vec::new(), Vec::new());
                for item in &self.args {
                    arg_name.push(format_ident!("{}", item.var));
                    arg_var.push(component_var(item.comp.idx));
                }

                // Expression to assign vars
                let new = quote!(
                    #ty {
                        #eid_var: #k
                        #(,#arg_name: #arg_var)*
                    }
                );

                match first_arg {
                    Some(_) => {
                        let var_fn = (0..var.len()).map_vec_into(|i| match i {
                            0 => quote!(|_, v| v),
                            i => {
                                let tmps = (0..i).map_vec_into(|i| format_ident!("v{i}"));
                                let tmp_n = format_ident!("v{}", i);
                                quote!(|(#(#tmps),*), #tmp_n| (#(#tmps,)* #tmp_n))
                            }
                        });
                        let var_iter = args.map_vec(|item| {
                            get_fn_name(
                                match item.comp.args.is_singleton {
                                    true => "get_vec",
                                    false => "iter",
                                },
                                item.is_mut,
                            )
                        });
                        quote!(
                            #(let #v = #var_intersect(#v.into_iter(), #comps_var.#var.#var_iter(), #var_fn);)*
                            let #v = #v.into_iter().map(|(#k, (#(#var),*))| #new).collect();
                        )
                    }
                    // No args
                    None => quote!(let #v = #v.into_iter().map(|(#k, _)| #new).collect();),
                }
            }
        }
    }

    fn codegen_copy(other: syn::Ident, ty: &syn::Path, vars: Vec<syn::Ident>) -> TokenStream {
        let eid = &CODEGEN_IDENTS.eid_var;
        let c = quote!(
            #ty {
                #eid: #other.#eid
                #(,#vars: #other.#vars)*
            }
        );
        c
    }

    fn codegen_copy_vec(other: syn::Ident, ty: &syn::Path, vars: Vec<syn::Ident>) -> TokenStream {
        let v = format_ident!("v");
        let copy = Self::codegen_copy(v.clone(), ty, vars);
        let c = quote!(#other.iter().map(|#v| #copy).collect());
        c
    }

    // Param 1: Vec<(cs function arg, cs, cs type)>
    pub fn codegen_build_sets(
        component_sets: &Vec<BuildSetsArg>,
        intersect: &syn::Path,
        intersect_opt: &syn::Path,
    ) -> BuildSetsResult {
        let CodegenIdents {
            eids_var,
            cfoo_var: comps_var,
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
        let get_vals = c_sets.iter().zip(var.iter()).map_vec_into(|(cs, v)| {
            cs.cs
                .codegen_get_vals(&cs.fn_arg, v.clone(), &cs.ty, intersect, intersect_opt)
        });

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
            singletons: c_sets.filter_map_vec(|cs| {
                match cs.cs.has_singleton() && !cs.fn_arg.is_vec {
                    true => Some(component_set_var(cs.fn_arg.idx).quote()),
                    false => None,
                }
            }),
        }
    }
}
