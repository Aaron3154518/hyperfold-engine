#![feature(proc_macro_span)]
#![feature(slice_group_by)]

extern crate proc_macro;
use std::path::PathBuf;
use std::{collections::HashSet, io::Write};

use ecs_macros::structs::ComponentArgs;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use regex::Regex;
use syn::parse_macro_input;

mod parse;
use parse::{Component, Event, Input, Service};

struct Out {
    f: std::fs::File,
}

impl Out {
    pub fn new(f: &'static str, app: bool) -> Self {
        Self {
            f: std::fs::OpenOptions::new()
                .create(true)
                .append(app)
                .write(true)
                .truncate(!app)
                .open(f)
                .expect(format!("Could not open {}", f).as_str()),
        }
    }

    pub fn write(&mut self, s: String) {
        self.f
            .write(s.as_bytes())
            .expect("Could not write to out.txt");
    }
}

#[proc_macro_attribute]
pub fn component(_input: TokenStream, item: TokenStream) -> TokenStream {
    let mut strct = parse_macro_input!(item as syn::ItemStruct);
    strct.vis = syn::parse_quote!(pub);

    quote!(#strct).into()
}

#[proc_macro_attribute]
pub fn system(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let fun = parse_macro_input!(item as syn::ItemFn);

    // Get service functions
    let services = std::env::var("SERVICES")
        .unwrap_or_else(|_| "SERVICES".to_string())
        .split(" ")
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    // Extract function paths
    let regex = Regex::new(r"\w+(::\w+)*").expect("msg");
    let s_names = services
        .iter()
        .map(|s| match regex.find(s) {
            Some(m) => m.as_str().split("::").collect::<Vec<_>>(),
            None => Vec::new(),
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    // Match this function to system functions
    let path = fun.sig.ident.span().unwrap().source_file().path();
    if let Some(dir) = path.parent() {
        let mut no_ext = PathBuf::new();
        no_ext.push(dir);
        no_ext.push(path.file_stem().expect("No file prefix"));
        let mut idxs = (0..s_names.len()).collect::<Vec<_>>();
        // Repeatedly prune impossible paths
        for (i, p) in no_ext.iter().enumerate() {
            let p = p
                .to_str()
                .expect(format!("Could not convert path segment to string: {:#?}", p).as_str());
            idxs.retain(|j| match s_names.get(*j) {
                Some(s) => match s.get(i) {
                    Some(s) => (*s == "crate" && p == "src") || *s == p,
                    None => false,
                },
                None => false,
            });
        }
        // Find this function in the possible paths
        let n = no_ext.iter().count();
        idxs.retain(|j| match s_names.get(*j) {
            Some(s) => {
                s.len() == n + 1
                    && match s.get(n) {
                        Some(s) => *s == fun.sig.ident.to_string(),
                        None => false,
                    }
            }
            None => false,
        });
        if let Some(i) = idxs.first() {
            if let Some(s) = services.get(*i) {
                // Parse function argument types
                let types = Regex::new(r"\w+:[01]:(?P<type>\d+)(,|\)$)")
                    .expect("Could not construct regex")
                    .captures_iter(s)
                    .filter_map(|m| match m.name("type") {
                        Some(t) => Some(t.as_str().parse::<usize>().expect("Could not parse type")),
                        None => {
                            println!("Could not match type");
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                // Argsort the types
                let mut idxs: Vec<usize> = (0..types.len()).collect();
                idxs.sort_by_key(|&i| &types[i]);

                // Re-order the function arguments
                let a = fun.sig.inputs.iter().collect::<Vec<_>>();
                let mut puncs = syn::punctuated::Punctuated::<_, syn::token::Comma>::new();
                puncs.extend(idxs.iter().map(|i| a[*i].clone()));
                // fun.sig.inputs = puncs;
            }
        }
    }

    let mut out = Out::new("out4.txt", true);
    out.write(format!("{:#?}\n", fun.sig.inputs));

    quote!(#fun).into()
}

#[proc_macro_attribute]
pub fn event(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut out = Out::new("out_en.txt", true);

    let mut en = parse_macro_input!(item as syn::ItemEnum);

    let mut derives = HashSet::new();
    for a in en.attrs.iter() {
        out.write(format!("{:#?}\n", a));
        match &a.meta {
            syn::Meta::List(l) => {
                if let Some(_) = l.path.segments.iter().find(|s| s.ident == "derive") {
                    l.parse_nested_meta(|l| {
                        for s in l.path.segments.iter() {
                            derives.insert(s.ident.to_string());
                        }
                        Ok(())
                    })
                    .expect("Could not parse nested meta");
                }
            }
            _ => (),
        }
    }
    let derives = ["PartialEq", "Eq"]
        .iter()
        .filter_map(|s| {
            if derives.contains(*s) {
                None
            } else {
                Some(format_ident!("{}", s))
            }
        })
        .collect::<Vec<_>>();

    en.vis = syn::parse_quote!(pub);

    let name = en.ident.to_owned();
    // Split variants into unit and field variants and get indices for each
    let (mut unit_vs, mut unit_idxs, mut field_vs, mut field_idxs) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (i, v) in en.variants.iter().enumerate() {
        let (vs, idxs) = match v.fields {
            syn::Fields::Unit => (&mut unit_vs, &mut unit_idxs),
            _ => (&mut field_vs, &mut field_idxs),
        };
        vs.push(v.ident.to_owned());
        idxs.push(
            syn::parse_str::<syn::LitInt>(format!("{}", i).as_str())
                .expect(format!("Could not parse int literal: {}", i).as_str()),
        );
    }

    // Get full list of indices
    let idxs = (0..en.variants.len())
        .map(|i| {
            syn::parse_str::<syn::LitInt>(format!("{}", i).as_str())
                .expect(format!("Could not parse int literal: {}", i).as_str())
        })
        .collect::<Vec<_>>();

    let code = quote!(
        #[derive(#(#derives),*)]
        #en

        impl #name {
            pub fn get_idx(i: usize) -> &'static crate::ecs::event::TypeIdx {
                static TYPE_IDXS: [crate::ecs::event::TypeIdx; 3] = [
                    #(crate::ecs::event::TypeIdx::new::<#name>(#idxs),)*
                ];
                &TYPE_IDXS[i]
            }

            pub fn to_idx(&self) -> &'static crate::ecs::event::TypeIdx {
                Self::get_idx(
                    match self {
                        #(Self::#unit_vs => #unit_idxs,)*
                        #(Self::#field_vs(..) => #field_idxs,)*
                    }
                )
            }
        }

        impl std::hash::Hash for #name {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.to_idx().hash(state);
            }
        }
    );
    out.write(format!("{:#?}\n", code.to_string()));

    code.into()
}

#[proc_macro]
pub fn component_manager(input: TokenStream) -> TokenStream {
    let mut f = Out::new("out3.txt", false);

    // Components
    let components = Component::parse(std::env::var("COMPONENTS").expect("COMPONENTS"));

    // Find the EventManager
    let c_eb = Component::find(&components, "crate::ecs::event::EventBus")
        .expect("Could not find EventBus")
        .var
        .to_owned();
    let c_ce = Component::find(&components, "crate::ecs::event::CurrEvent")
        .expect("Could not find CurrEvent")
        .var
        .to_owned();

    // Services
    let mut services = Service::parse(std::env::var("SERVICES").expect("SERVICES"));

    // Partition by signature
    services.sort_by(|s1, s2| s1.cmp(s2));
    let services = services
        .group_by(|v1, v2| v1.eq_sig(v2))
        .map(|s| s.to_vec())
        .collect::<Vec<_>>();

    // Get service names and signatures for each signature bin
    let (s_names, s_args) = services
        .iter()
        .map(|v| {
            (
                v.iter().map(|s| s.get_path()).collect::<Vec<_>>(),
                v.first().map_or(Vec::new(), |s| s.get_args(&components)),
            )
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    // Split args types by component type
    let (mut s_carg_vs, mut s_carg_ts, mut s_garg_vs, mut s_garg_ts) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    s_args.into_iter().for_each(|v| {
        s_carg_vs.push(Vec::new());
        s_carg_ts.push(Vec::new());
        s_garg_vs.push(Vec::new());
        s_garg_ts.push(Vec::new());
        v.group_by(|c1, c2| c1.arg_type == c2.arg_type)
            .for_each(|v| {
                let (vs, ts) = match v.first().map_or(ComponentArgs::None, |c| c.arg_type) {
                    ComponentArgs::None => (&mut s_carg_vs, &mut s_carg_ts),
                    ComponentArgs::Global => (&mut s_garg_vs, &mut s_garg_ts),
                };
                vs.pop();
                vs.push(v.iter().map(|c| c.var.to_owned()).collect::<Vec<_>>());
                ts.pop();
                ts.push(v.iter().map(|c| c.ty.to_owned()).collect::<Vec<_>>());
            });
    });

    // Partition components into types
    let (mut c_vars, mut c_types, mut g_vars, mut g_types) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    components
        .group_by(|c1, c2| c1.arg_type == c2.arg_type)
        .for_each(|v| {
            let (vs, ts) = match v.first().map_or(ComponentArgs::None, |c| c.arg_type) {
                ComponentArgs::None => (&mut c_vars, &mut c_types),
                ComponentArgs::Global => (&mut g_vars, &mut g_types),
            };
            vs.extend(v.iter().map(|c| c.var.to_owned()).collect::<Vec<_>>());
            ts.extend(v.iter().map(|c| c.ty.to_owned()).collect::<Vec<_>>());
        });

    // Events
    let events = Event::parse(std::env::var("EVENTS").expect("EVENTS"));

    // Parse variant paths
    let (s_event_paths, s_event_idxs) = services
        .iter()
        .map(|v| {
            v.iter()
                .map(|s| s.get_events(&events))
                .unzip::<_, _, Vec<_>, Vec<_>>()
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let (sm, cm) = parse_macro_input!(input as Input).get();

    let code = quote!(
        use ecs_macros::*;
        manager!(#cm,
            c(#(#c_vars, #c_types),*),
            g(#(#g_vars, #g_types),*)
        );
        systems!(#sm, #cm, EFoo, #c_eb, #c_ce,
            #(
                ((#(#s_names[#(#s_event_paths, #s_event_idxs),*]),*),
                c(#(#s_carg_vs, #s_carg_ts),*),
                g(#(#s_garg_vs, #s_garg_ts),*))
            ),*
        );
    );

    // Open out.txt to print stuff
    f.write(format!("Code:\n{}\n", code));

    code.into()
}

#[proc_macro]
pub fn event_manager(input: TokenStream) -> TokenStream {
    let mut out = Out::new("out_em.txt", false);

    // Events
    let events = Event::parse(std::env::var("EVENTS").expect("EVENTS"));

    // Parse out event paths and aliases
    let (ev_ts, ev_vs) = events
        .iter()
        .map(|e| (e.get_path(), e.get_type()))
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let em = parse_macro_input!(input as syn::Ident);

    let code = quote!(
        events!(#em, #(#ev_vs(#ev_ts)),*);
    );

    out.write(format!("{}", code.to_string()));

    code.into()
}
