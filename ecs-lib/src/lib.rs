#![feature(proc_macro_span)]
#![feature(slice_group_by)]

extern crate proc_macro;
use std::io::Write;

use ecs_macros::structs::ComponentTypes;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{parse_macro_input, parse_quote};

mod parse;
use parse::{Component, EventMod, Input, System};

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

// TODO: Const can't be taken as mut
#[proc_macro_attribute]
pub fn component(input: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as ComponentType);
    if args.is_dummy {
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

#[proc_macro_attribute]
pub fn system(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut fun = parse_macro_input!(item as syn::ItemFn);
    fun.vis = parse_quote!(pub);

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
    let components = Component::parse(
        std::env::var("COMPONENTS").expect("COMPONENTS"),
        ComponentTypes::None,
    );
    let globals = Component::parse(
        std::env::var("GLOBALS").expect("GLOBALS"),
        ComponentTypes::Global,
    );

    // Find specific components
    let [g_eb, g_ev, g_rs, g_cm, g_tr] = [
        "crate::EFoo",
        "crate::utils::event::Event",
        "crate::asset_manager::RenderSystem",
        "crate::CFoo",
        "crate::ecs::entity::EntityTrash",
    ]
    .map(|s| Component::find(&globals, s).expect(s).var.to_owned());

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

    // Systems
    let systems = System::parse(std::env::var("SYSTEMS").expect("SYSTEMS"));

    // Get system events
    let (systems, s_ev_varis) = systems
        .into_iter()
        .filter_map(|s| {
            s.event
                .as_ref()
                .map(|e| {
                    e_varis
                        .get(e.e_idx)
                        .and_then(|v_v| v_v.get(e.v_idx).map(|v| v.to_owned()))
                        .expect("Could not find system event")
                })
                .map(|t| (s, t))
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    // Get system names and signatures
    let (s_names, s_args) = systems
        .iter()
        .map(|s| (s.get_path(), s.get_args(&components, &globals)))
        .unzip::<_, _, Vec<_>, Vec<_>>();

    // Partition components into types
    let (c_vars, c_types) = components
        .into_iter()
        .map(|c| (c.var, c.ty))
        .unzip::<_, _, Vec<_>, Vec<_>>();
    let (g_vars, g_types) = globals
        .into_iter()
        .map(|g| (g.var, g.ty))
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let (sm, cm, gm, em) = parse_macro_input!(input as Input).get();

    let funcs = s_names
        .iter()
        .zip(s_args.iter())
        .map(|(f, args)| args.to_quote(f, &cm, &gm, &em))
        .collect::<Vec<_>>();

    let code = quote!(
        use ecs_macros::*;
        events!(#em,
            #(#((#e_strcts, #e_varis, #e_vars)),*),*
        );
        c_manager!(#cm, #(#c_vars, #c_types),*);
        g_manager!(#gm, #(#g_vars, #g_types),*);
        systems!(#sm, #cm, #gm, #em,
            #g_eb, #g_ev, #g_rs, #g_cm, #g_tr,
            #(#s_ev_varis, #funcs),*
        );
    );

    // Open out.txt to print stuff
    f.write(format!("Code:\n{}\n", code));

    code.into()
}
