#![feature(proc_macro_span)]
#![feature(slice_group_by)]

extern crate proc_macro;
use std::io::Write;

use ecs_macros::structs::ComponentArgs;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{parse_macro_input, parse_quote, spanned::Spanned};

mod parse;
use parse::{find, Component, EventMod, Input, Service};

use ecs_macros::structs::ComponentType;

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
pub fn component(input: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as ComponentType);
    if args.is_dummy() {
        return quote!().into();
    }

    let input = parse_macro_input!(item as syn::Item);
    match input {
        syn::Item::Struct(mut s) => {
            s.vis = syn::parse_quote!(pub);
            quote!(#s)
        }
        syn::Item::Type(mut t) => {
            t.vis = syn::parse_quote!(pub);
            quote!(#t)
        }
        _ => panic!("Only structs and types can be components"),
    }
    .into()
}

// TODO: Don't access global data
#[proc_macro_attribute]
pub fn system(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut fun = parse_macro_input!(item as syn::ItemFn);

    fun.vis = parse_quote!(pub);

    // Services
    let services = Service::parse(std::env::var("SERVICES").expect("SERVICES"));

    // Extract function paths
    let s_names = services.iter().map(|s| s.path.to_vec()).collect::<Vec<_>>();

    // Match this function to system functions
    if let Some(s) = services
        .get(find(fun.span(), fun.sig.ident.to_string(), s_names).expect("Could not find service"))
    {
        // Re-order the function arguments
        let args = std::mem::replace(&mut fun.sig.inputs, syn::punctuated::Punctuated::new());
        fun.sig.inputs =
            s.get_arg_idxs()
                .iter()
                .fold(syn::punctuated::Punctuated::new(), |mut p, &idx| {
                    p.push(args[idx].to_owned());
                    p
                });
    }

    quote!(#fun).into()
}

#[proc_macro_attribute]
pub fn event(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut out = Out::new("out_en.txt", true);

    let en = parse_macro_input!(item as syn::ItemEnum);

    let name = en.ident;
    let strcts = en
        .variants
        .into_iter()
        .map(|mut v| {
            let mut semi = Some(syn::token::Semi(Span::call_site()));
            // make fields public
            match &mut v.fields {
                syn::Fields::Named(n) => {
                    for f in n.named.iter_mut() {
                        f.vis = parse_quote!(pub)
                    }
                }
                syn::Fields::Unnamed(u) => {
                    for f in u.unnamed.iter_mut() {
                        f.vis = parse_quote!(pub)
                    }
                }
                syn::Fields::Unit => semi = None,
            };
            syn::ItemStruct {
                attrs: Vec::new(),
                vis: syn::parse_quote!(pub),
                struct_token: syn::parse_quote!(struct),
                ident: v.ident,
                generics: syn::Generics {
                    lt_token: None,
                    params: syn::punctuated::Punctuated::new(),
                    gt_token: None,
                    where_clause: None,
                },
                fields: v.fields,
                semi_token: semi,
            }
        })
        .collect::<Vec<_>>();

    let code = quote!(
        pub mod #name {
            use super::*;
            #(
                #[derive(Debug)]
                #strcts
            )*
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
    let c_eb = Component::find(&components, "crate::EFoo")
        .expect("Could not find EFoo")
        .var
        .to_owned();
    let c_ev = Component::find(&components, "crate::utils::event::Event")
        .expect("Could not find Event")
        .var
        .to_owned();
    let c_rs = Component::find(&components, "crate::asset_manager::RenderSystem")
        .expect("Could not find RenderSystem")
        .var
        .to_owned();

    // Events
    let events = EventMod::parse(std::env::var("EVENTS").expect("EVENTS"));

    let mut cnt = 0;
    let (e_strcts, e_varis, e_vars) = events.iter().enumerate().fold(
        (Vec::new(), Vec::new(), Vec::new()),
        |(mut strcts, mut varis, mut vars), (i, em)| {
            let p = em.get_path();
            let (new_strcts, new_varis, new_vars) = em.events.iter().enumerate().fold(
                (Vec::new(), Vec::new(), Vec::new()),
                |(mut strcts, mut varis, mut vars), (j, e)| {
                    let mut p = p.clone();
                    p.segments.push(syn::PathSegment {
                        ident: format_ident!("{}", e),
                        arguments: syn::PathArguments::None,
                    });
                    strcts.push(p);
                    vars.push(format_ident!("e{}_{}", i, j));
                    varis.push(format_ident!("_{}", format!("{}", cnt)));
                    cnt += 1;
                    (strcts, varis, vars)
                },
            );
            strcts.push(new_strcts);
            varis.push(new_varis);
            vars.push(new_vars);
            (strcts, varis, vars)
        },
    );

    // Services
    let mut services = Service::parse(std::env::var("SERVICES").expect("SERVICES"));

    // Partition by signature and filter out services that have no event
    services.sort_by(|s1, s2| s1.cmp(s2));
    let (services, s_events) = services
        .group_by(|v1, v2| v1.eq_sig(v2))
        .map(|s| s.into_iter().collect::<Vec<_>>())
        .filter_map(|v_s| {
            v_s.first()
                .and_then(|s| {
                    s.event.as_ref().map(|e| {
                        e_strcts
                            .get(e.e_idx)
                            .zip(e_varis.get(e.e_idx))
                            .and_then(|(v_s, v_v)| {
                                v_s.get(e.v_idx)
                                    .zip(v_v.get(e.v_idx))
                                    .map(|(s, v)| (s.to_owned(), v.to_owned()))
                            })
                            .expect("Could not find service event")
                    })
                })
                .map(|e| (v_s, e))
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();
    let (s_ev_paths, s_ev_varis) = s_events.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();

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
                    _ => return,
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
                _ => return,
            };
            vs.extend(v.iter().map(|c| c.var.to_owned()).collect::<Vec<_>>());
            ts.extend(v.iter().map(|c| c.ty.to_owned()).collect::<Vec<_>>());
        });

    let (sm, cm, em) = parse_macro_input!(input as Input).get();

    let code = quote!(
        use ecs_macros::*;
        events!(#em,
            #(#((#e_strcts, #e_varis, #e_vars)),*),*
        );
        manager!(#cm,
            c(#(#c_vars, #c_types),*),
            g(#(#g_vars, #g_types),*)
        );
        systems!(#sm, #cm, #em, #c_eb, #c_ev, #c_rs,
            #(
                ((#(#s_names),*),
                e(#s_ev_paths, #s_ev_varis),
                c(#(#s_carg_vs, #s_carg_ts),*),
                g(#(#s_garg_vs, #s_garg_ts),*))
            ),*
        );
    );

    // Open out.txt to print stuff
    f.write(format!("Code:\n{}\n", code));

    code.into()
}
