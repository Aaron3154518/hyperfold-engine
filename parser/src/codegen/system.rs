use std::array;
use std::collections::HashSet;
use std::hash::Hash;

use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use regex::Regex;

use crate::{
    codegen::{
        idents::Idents,
        util::{arr_to_path, string_to_type, type_to_type, vec_to_path},
    },
    resolve::{
        ast_items::{self, ItemsCrate},
        ast_paths::{EngineGlobals, EngineIdents, EngineTraits, ExpandEnum, GetPaths, Paths},
    },
    validate::{
        ast_system::SystemValidate,
        constants::{component_var, event_variant, global_var, EID},
    },
};
use shared::util::{Call, Catch, Flatten, JoinMap, JoinMapInto, SplitCollect};

#[shared::macros::expand_enum]
pub enum LabelType {
    And,
    Or,
    Nand,
    Nor,
}

impl From<&str> for LabelType {
    fn from(value: &str) -> Self {
        let value = value.trim_start_matches("l");
        if value.starts_with("&") {
            Self::And
        } else if value.starts_with("|") {
            Self::Or
        } else if value.starts_with("!&") {
            Self::Nand
        } else if value.starts_with("!|") {
            Self::Nor
        } else {
            panic!("Invalid label argument: {value}")
        }
    }
}

#[derive(Clone, Debug)]
pub enum ContainerArg {
    EntityId,
    Component(usize, usize, bool),
}

pub enum FnArg {
    EntityId,
    Component(syn::Ident),
    Global(syn::Ident),
    Event(syn::Ident),
    Label(LabelType, HashSet<syn::Ident>),
    Container(Vec<ContainerArg>),
}

pub struct System {
    pub name: TokenStream,
    args: Vec<TokenStream>,
    c_args: Vec<syn::Ident>,
    g_args: Vec<syn::Ident>,
    event: syn::Ident,
    and_labels: HashSet<syn::Ident>,
    or_labels: Vec<HashSet<syn::Ident>>,
    nor_labels: HashSet<syn::Ident>,
    nand_labels: Vec<HashSet<syn::Ident>>,
    // Includes reference and mutability
    v_types: Vec<ContainerArg>,
    is_vec: bool,
    pub is_init: bool,
}

impl System {
    pub fn new() -> Self {
        Self {
            name: quote!(),
            args: Vec::new(),
            c_args: Vec::new(),
            g_args: Vec::new(),
            event: format_ident!("_"),
            and_labels: HashSet::new(),
            or_labels: Vec::new(),
            nor_labels: HashSet::new(),
            nand_labels: Vec::new(),
            v_types: Vec::new(),
            is_vec: false,
            is_init: false,
        }
    }

    pub fn from(
        system: &ast_items::System,
        cr_path: &syn::Path,
        paths: &Paths,
        crates: &Vec<ItemsCrate>,
    ) -> Self {
        let mut sys = System::new();

        let [gfoo, e_ident, v_ident, eid] =
            [Idents::GenGFoo, Idents::GenE, Idents::GenV, Idents::GenEid].map(|i| i.to_ident());

        let mut validate = SystemValidate::new();
        let path = vec_to_path(system.path.path[1..].to_vec());
        sys.name = quote!(#cr_path::#path);
        sys.is_init = system.attr_args.is_init;
        sys.args = system.args.map_vec(|arg| {
            match arg
                // Entity ID
                .to_eid(paths)
                .map(|_| {
                    arg.validate_ref(1, &mut validate.errs);
                    arg.validate_mut(false, &mut validate.errs);
                    validate.add_eid();
                    quote!(#eid)
                })
                // Component
                .or_else(|| {
                    arg.to_component(crates).map(|(cr_i, c_i, _)| {
                        arg.validate_ref(1, &mut validate.errs);
                        validate.add_component(arg, (cr_i, c_i));
                        let var = format_ident!("{}", component_var(cr_i, c_i));
                        let tt = quote!(#var);
                        sys.c_args.push(var);
                        tt
                    })
                })
                // Global
                .or_else(|| {
                    arg.to_global(crates).map(|(cr_i, g_i, g_args)| {
                        arg.validate_ref(1, &mut validate.errs);
                        if g_args.is_const {
                            arg.validate_mut(false, &mut validate.errs);
                        }
                        validate.add_global(arg, (cr_i, g_i));
                        let var = format_ident!("{}", global_var(cr_i, g_i));
                        let tt = quote!(&mut #gfoo.#var);
                        sys.g_args.push(var);
                        tt
                    })
                })
                // Event
                .or_else(|| {
                    arg.to_event(crates).map(|(cr_i, e_i)| {
                        arg.validate_ref(1, &mut validate.errs);
                        arg.validate_mut(false, &mut validate.errs);
                        validate.add_event(arg);
                        sys.event = format_ident!("{}", event_variant(cr_i, e_i));
                        quote!(#e_ident)
                    })
                })
                // Trait
                .or_else(|| {
                    // This assumes the traits are all at the beginning of the globals list
                    arg.to_trait(crates).map(|(cr_i, g_i)| {
                        arg.validate_ref(1, &mut validate.errs);
                        validate.add_global(arg, (cr_i, g_i));
                        let var = format_ident!("{}", global_var(cr_i, g_i));
                        let tt = quote!(&mut #gfoo.#var);
                        sys.g_args.push(var);
                        tt
                    })
                })
                // Label
                .or_else(|| {
                    arg.to_label(paths).map(|(l_ty, args)| {
                        arg.validate_ref(0, &mut validate.errs);
                        let args = args
                            .iter()
                            .map(|a| {
                                a.to_component(crates).map_or_else(
                                    || {
                                        &mut validate.errs.push(format!(
                                            "Label expects Component type, found: {}",
                                            a
                                        ));
                                        format_ident!("_")
                                    },
                                    |(cr_i, c_i, _)| format_ident!("{}", component_var(cr_i, c_i)),
                                )
                            })
                            .collect::<HashSet<_>>();
                        match l_ty {
                            LabelType::And => sys.and_labels.extend(args.into_iter()),
                            LabelType::Or => sys.or_labels.push(args),
                            LabelType::Nand => sys.nand_labels.push(args),
                            LabelType::Nor => sys.nor_labels.extend(args.into_iter()),
                        }
                        quote!(std::marker::PhantomData)
                    })
                })
                // Container
                .or_else(|| {
                    arg.to_container(paths).map(|args| {
                        arg.validate_ref(0, &mut validate.errs);
                        validate.add_container(arg);
                        args.join_map(
                            |a| {
                                a.to_eid(paths)
                                    .map(|_| {
                                        a.validate_ref(1, &mut validate.errs);
                                        a.validate_mut(false, &mut validate.errs);
                                        EID.to_string()
                                    })
                                    .or_else(|| {
                                        a.to_component(crates).map(|(cr_i, c_i, _)| {
                                            a.validate_ref(1, &mut validate.errs);
                                            format!(
                                                "{}{}",
                                                if a.mutable { "m" } else { "" },
                                                component_var(cr_i, c_i)
                                            )
                                        })
                                    })
                                    .map_or_else(
                                        || {
                                            &mut validate.errs.push(format!(
                                        "Container expects Component or Entity ID type, found: {}",
                                        a
                                    ));
                                            String::new()
                                        },
                                        |s| s,
                                    )
                            },
                            "-",
                        );
                        sys.is_vec = true;
                        (sys.c_args, sys.v_types) = args.unzip_vec(|a| {
                            a.to_eid(paths)
                                .map(|_| {
                                    a.validate_ref(1, &mut validate.errs);
                                    a.validate_mut(false, &mut validate.errs);
                                    (Idents::GenEids.to_ident(), ContainerArg::EntityId)
                                })
                                .or_else(|| {
                                    a.to_component(crates).map(|(cr_i, c_i, _)| {
                                        a.validate_ref(1, &mut validate.errs);
                                        let var = format_ident!("{}", component_var(cr_i, c_i));
                                        (var, ContainerArg::Component(cr_i, c_i, a.mutable))
                                    })
                                })
                                .map_or_else(
                                    || {
                                        validate.errs.push(format!(
                                            "Container expects Component or Entity ID type, found: {}",
                                            a
                                        ));
                                        (format_ident!("_"), ContainerArg::EntityId)
                                    },
                                    |t| t,
                                )
                        });
                        quote!(#v_ident)
                    })
                }) {
                Some(a) => a,
                None => {
                    validate
                        .errs
                        .push(format!("Argument: \"{}\" is not a known type", arg));
                    quote!()
                }
            }
        });
        validate.validate(&system.attr_args);
        if !validate.errs.is_empty() {
            panic!(
                "\n\nIn system: \"{}()\"\n{}\n\n",
                system.path.path.join("::"),
                validate.errs.join("\n")
            )
        }
        sys
    }

    fn check_labels(&mut self) {
        // Components are implicitly part of AND
        self.and_labels
            .extend(self.c_args.iter().map(|i| i.to_owned()));

        // NOR can't include the label, but AND must include the label
        if !self.and_labels.is_disjoint(&self.nor_labels) {
            panic!(
                "{}\n{}",
                "A label is in both AND and NOR. The label condition cannot be satisfied",
                "Note that all components are implicitly AND labels"
            )
        }

        // AND must have it, so OR is automatically satisfied
        self.or_labels
            .drain_filter(|ors| ors.is_empty() || !self.and_labels.is_disjoint(ors));
        // NOR must not have it, so OR is automatically checked
        for ors in self.or_labels.iter_mut() {
            ors.drain_filter(|c| self.nor_labels.contains(c));
            // NOR must have none, but OR must have at least one
            if ors.is_empty() {
                panic!(
                    "All labels in at least one OR set are also in NOR. The label condition cannot be satisfied"
                )
            }
        }

        // NOR must not have it, so NAND is automatically satisfied
        self.nand_labels
            .drain_filter(|nands| nands.is_empty() || !self.nor_labels.is_disjoint(nands));
        for nands in self.nand_labels.iter_mut() {
            nands.drain_filter(|c| self.and_labels.contains(c));
            // AND must have all, but NAND must not have at least one
            if nands.is_empty() {
                panic!(
                        "{}\n{}",
                        "All labels in at least on NAND set are also in AND. The label condition cannot be satisfied",
                        "Note that all components are implicitly AND labels"
                    )
            }
        }

        // Remove all components from AND
        self.and_labels.drain_filter(|c| self.c_args.contains(c));
    }

    fn quote_labels(&self, body: TokenStream) -> TokenStream {
        let [cfoo, eid] = [Idents::GenCFoo, Idents::GenEid].map(|i| i.to_ident());

        let (ands, nors) = (self.and_labels.iter(), self.nor_labels.iter());

        let mut checks = Vec::new();
        for (i, v) in [&self.and_labels, &self.nor_labels].iter().enumerate() {
            if !v.is_empty() {
                let vs = v.iter();
                let not = if i == 0 { quote!() } else { quote!(!) };
                checks.push(quote!(#(#not #cfoo.#vs.contains_key(#eid))&&*));
            }
        }
        for (i, v) in [&self.or_labels, &self.nand_labels].iter().enumerate() {
            let not = if i == 0 { quote!() } else { quote!(!) };
            checks.append(&mut v.map_vec(|v| {
                let vs = v.iter();
                quote!(#(#not #cfoo.#vs.contains_key(#eid))||*)
            }));
        }

        if checks.is_empty() {
            body
        } else {
            quote!(
                if (#((#checks))&&*) {
                    #body
                }
            )
        }
    }

    pub fn to_quote(
        &self,
        engine_crate_path: &syn::Path,
        components: &Vec<Vec<TokenStream>>,
    ) -> TokenStream {
        let f = &self.name;
        let args = &self.args;

        let body = if self.c_args.is_empty() {
            quote!(#f(#(#args),*))
        } else if !self.is_vec {
            let c_args = &self.c_args;
            let label_checks = self.quote_labels(quote!(#f(#(#args),*)));

            let eid = Idents::GenEid.to_ident();

            let [get_keys, intersect_keys] = [EngineIdents::GetKeys, EngineIdents::IntersectKeys]
                .map(|i| vec_to_path(i.path_stem()));

            quote!(
                for #eid in #engine_crate_path::#intersect_keys(&mut [#(#engine_crate_path::#get_keys(&cfoo.#c_args)),*]).iter() {
                    if let (#(Some(#c_args),)*) = (#(cfoo.#c_args.get_mut(#eid),)*) {
                        #label_checks
                    }
                }
            )
        } else {
            let eid = EngineIdents::Entity.to_path();

            // Container argument types
            let v_types = self
                .v_types
                .iter()
                .map(|a| match a {
                    ContainerArg::EntityId => quote!(&#engine_crate_path::#eid),
                    ContainerArg::Component(cr_idx, c_idx, m) => {
                        let c_ty = &components[*cr_idx][*c_idx];
                        let mut_tok = if *m { quote!(mut) } else { quote!() };
                        quote!(&#mut_tok #c_ty)
                    }
                })
                .collect::<Vec<_>>();
            // Get first argument to initialize the result hashmap
            let arg = self.c_args.first().expect("No first component");
            let nones = ["None"].repeat(self.v_types.len() - 1).join(",");
            let (iter, tuple_init) = match self.v_types.first().expect("No first vector types") {
                ContainerArg::EntityId => ("iter", format!("|k| (k, (None, {}))", nones)),
                ContainerArg::Component(_, _, m) => (
                    if *m { "iter_mut" } else { "iter" },
                    format!("|(k, v)| (k, (Some(v), {}))", nones),
                ),
            };
            let iter = format_ident!("{}", iter);
            let tuple_init = syn::parse_str::<syn::ExprClosure>(tuple_init.as_str())
                .expect("Could not parse tuple init closure");

            let [cfoo, v, eid] =
                [Idents::GenCFoo, Idents::GenV, Idents::GenEid].map(|i| i.to_ident());

            // Intersect with tail args
            let intersect_stmts = self.c_args[1..]
                .iter()
                .zip(self.v_types[1..].iter())
                .enumerate()
                .filter_map(|(mut i, (a, ty))| match ty {
                    ContainerArg::EntityId => None,
                    ContainerArg::Component(_, _, m) => {
                        let intersect = vec_to_path(
                            if *m {
                                EngineIdents::IntersectMut
                            } else {
                                EngineIdents::Intersect
                            }
                            .path_stem(),
                        );
                        let member: syn::Member = syn::parse_str(format!("{}", i + 1).as_str())
                            .catch(format!("Could not parse expression: {}", i + 1));
                        Some(quote!(
                            #engine_crate_path::#intersect(#v, &mut #cfoo.#a, |t| &mut t.#member)
                        ))
                    }
                })
                .collect::<Vec<_>>();

            // Contsruct final vector
            // v1, v2, ...
            // c_vars only contains v_i where i is not an eid
            let mut c_vars = Vec::new();
            // all_vars contains all v_i
            // all_args replaces eids with "k"
            let (all_vars, all_args) = self
                .c_args
                .iter()
                .zip(self.v_types.iter())
                .enumerate()
                .map(|(i, (_v, ty))| {
                    let v_i = format_ident!("v{}", i);
                    match ty {
                        ContainerArg::EntityId => (v_i, eid.to_owned()),
                        ContainerArg::Component(_, _, _) => {
                            c_vars.push(v_i.to_owned());
                            (v_i.to_owned(), v_i)
                        }
                    }
                })
                .unzip::<_, _, Vec<_>, Vec<_>>();

            let label_checks = self.quote_labels(quote!(return Some((#(#all_args,)*));));

            quote!(
                let mut #v = cfoo.#arg
                    .#iter()
                    .map(#tuple_init)
                    .collect::<std::collections::HashMap<_, (#(Option<#v_types>,)*)>>();
                #(#v = #intersect_stmts;)*
                let #v = #v
                    .into_iter()
                    .filter_map(|(#eid, (#(#all_vars,)*))| {
                        if let (#(Some(#c_vars),)*) = (#(#c_vars,)*) {
                            #label_checks
                        }
                        None
                    })
                    .collect::<Vec<_>>();
                #f(#(#args),*);
            )
        };

        let [cfoo_ty, efoo_ty] = [Idents::CFoo, Idents::EFoo].map(|i| i.to_ident());
        let gfoo_ty = Idents::GFoo.to_ident();
        let [cfoo, gfoo, efoo] =
            [Idents::GenCFoo, Idents::GenGFoo, Idents::GenEFoo].map(|i| i.to_ident());
        let add_event = EngineTraits::AddEvent.to_path();

        if self.is_init {
            quote!(
                (|#cfoo: &mut #cfoo_ty, #gfoo: &mut #gfoo_ty, #efoo: &mut #efoo_ty| {
                      #body
                })(&mut self.#cfoo, &mut self.#gfoo, &mut self.#efoo);
            )
        } else {
            let [e_ident, e] = [Idents::E, Idents::GenE].map(|i| i.to_ident());
            let event = &self.event;
            quote!(
                let f = |#cfoo: &mut #cfoo_ty, #gfoo: &mut #gfoo_ty, #efoo: &mut #efoo_ty| {
                    if let Some(#e) = #engine_crate_path::#add_event::get_event(#efoo) {
                        #body
                    }
                };
                self.add_system(#e_ident::#event, Box::new(f));
            )
        }
    }
}
