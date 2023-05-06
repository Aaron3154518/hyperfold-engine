#![feature(proc_macro_span)]
#![feature(slice_group_by)]
#![feature(drain_filter)]

extern crate proc_macro;

use hyperfold_shared::{
    macro_args::{ComponentMacroArgs, GlobalMacroArgs},
    paths::{
        COMPONENTS_MANAGER, COMPONENTS_TRAIT, EVENTS_MANAGER, EVENTS_TRAIT, GLOBALS_MANAGER,
        SYSTEMS_MANAGER,
    },
};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{parse_macro_input, parse_quote};

mod parse;
use parse::{
    component::Component,
    dependencies::Dependencies,
    event::EventMod,
    paths::*,
    system::System,
    util::{arr_to_path, Out},
};

use crate::parse::component::Global;

#[proc_macro_attribute]
pub fn component(input: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as ComponentMacroArgs);
    if args.is_dummy {
        return quote!().into();
    }

    let cm_tr = format_ident!("{}", COMPONENTS_TRAIT);
    let input = parse_macro_input!(item as syn::Item);
    match input {
        syn::Item::Struct(mut s) => {
            s.vis = syn::parse_quote!(pub);
            let name = &s.ident;
            if args.is_label {
                quote!(
                    #s

                    impl crate::LabelTrait for #name {
                        fn add_label(&self, cm: &mut dyn crate::#cm_tr, eid: crate::ecs::entities::Entity) {
                            cm.add_component(eid, Self)
                        }
                    }
                )
            } else {
                quote!(#s)
            }
        }
        _ => panic!("Only structs can be components"),
    }
    .into()
}

#[proc_macro_attribute]
pub fn global(input: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as GlobalMacroArgs);
    if args.is_dummy {
        return quote!().into();
    }

    let input = parse_macro_input!(item as syn::Item);
    match input {
        syn::Item::Struct(mut s) => {
            s.vis = syn::parse_quote!(pub);
            quote!(#s)
        }
        _ => panic!("Only structs can be globals"),
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

    quote!(
        pub mod #name {
            use super::*;
            #(
                #[derive(Debug)]
                #strcts
            )*
        }
    )
    .into()
}

fn main_component_manager(dir: String) -> proc_macro2::TokenStream {
    // Key structs/traits
    let (sm, cm, gm, em, cm_tr, em_tr) = (
        format_ident!("{}", SYSTEMS_MANAGER),
        format_ident!("{}", COMPONENTS_MANAGER),
        format_ident!("{}", GLOBALS_MANAGER),
        format_ident!("{}", EVENTS_MANAGER),
        format_ident!("{}", COMPONENTS_TRAIT),
        format_ident!("{}", EVENTS_TRAIT),
    );

    // Events
    let events = EventMod::parse(dir.to_string());

    let mut cnt = 0usize;
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
    let num_events = syn::LitInt::new(cnt.to_string().as_str(), Span::call_site());

    // Components
    let components = Component::parse(dir);
    // Globals
    let globals = Global::parse();

    // Systems
    let mut systems = System::parse();

    // Filter out init systems
    let init_systems = systems.drain_filter(|s| s.is_init).collect::<Vec<_>>();

    // Get system events and filter out systems with no events
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

    // Get system func bodies
    let funcs = systems
        .iter()
        .map(|s| {
            s.get_args(&components, &globals)
                .to_quote(&s.get_path(), &cm, &gm, &em)
        })
        .collect::<Vec<_>>();
    let init_funcs = init_systems
        .iter()
        .map(|s| {
            s.get_args(&components, &globals)
                .to_quote(&s.get_path(), &cm, &gm, &em)
        })
        .collect::<Vec<_>>();

    // Find specific globals
    let [g_event_manager, g_event, g_render_system, g_component_manager, g_entity_trash, g_screen, g_camera] =
        [
            &crate_(em.to_string().as_str())[..],
            &EVENT[..],
            &RENDER_SYSTEM[..],
            &crate_(cm.to_string().as_str())[..],
            &ENTITY_TRASH[..],
            &SCREEN[..],
            &CAMERA[..],
        ]
        .map(|s| {
            Global::find(&globals, s)
                .expect(format!("Could not find global: {}", s.join("::")).as_str())
                .var
                .to_owned()
        });

    // Partition components into types
    let (c_vars, c_types) = components
        .into_iter()
        .map(|c| (c.var, c.ty))
        .unzip::<_, _, Vec<_>, Vec<_>>();
    let (g_vars, g_types) = globals
        .into_iter()
        .map(|g| (g.var, g.ty))
        .unzip::<_, _, Vec<_>, Vec<_>>();

    // Dependencies
    let deps = Dependencies::parse()
        .into_iter()
        .map(|d| format_ident!("{}", d.name))
        .collect::<Vec<_>>();

    quote!(
        mod _engine {
            use crate::ecs::shared::*;
            use crate::ecs::entities::{Entity, EntityTrash};
            use crate::ecs::events::CoreEvent;
            events!(#em, #em_tr, (#(#deps),*), #num_events,
                #(#((#e_strcts, #e_varis, #e_vars)),*),*
            );
            c_manager!(#cm, #cm_tr, (#(#deps),*), #(#c_vars, #c_types),*);
            g_manager!(#gm, #(#g_vars, #g_types),*);
            systems!(#sm, #cm, #gm, #em,
                (
                    #g_event_manager, #g_event, #g_render_system,
                    #g_component_manager, #g_entity_trash,
                    #g_screen, #g_camera
                ),
                (#(#init_funcs),*), (#(#s_ev_varis, #funcs),*)
            );
        }
        use _engine::{#sm, #cm, #gm, #em, #cm_tr, #em_tr, LabelTrait};
    )
}

fn sub_component_manager(dir: String) -> proc_macro2::TokenStream {
    let deps = Dependencies::parse()
        .into_iter()
        .find(|d| d.name == dir)
        .expect(format!("Could not find dependencies for: {}", dir.to_string()).as_str());

    // Events
    let e_strcts = EventMod::parse(dir.to_string())
        .into_iter()
        .filter(|e| deps.contains(&e.dep))
        .map(|e| {
            let path = e.get_path();
            e.events
                .into_iter()
                .map(|v| {
                    let mut p = path.to_owned();
                    p.segments.push(syn::PathSegment {
                        ident: format_ident!("{}", v),
                        arguments: syn::PathArguments::None,
                    });
                    p
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    // Components
    let c_types = Component::parse(dir)
        .into_iter()
        .filter(|c| deps.contains(&c.dep))
        .map(|c| c.ty)
        .collect::<Vec<_>>();

    let (cm_tr, em_tr) = (
        format_ident!("{}", COMPONENTS_TRAIT),
        format_ident!("{}", EVENTS_TRAIT),
    );

    quote!(
        mod _engine {
            use crate::ecs::shared::*;
            use crate::ecs::entities::Entity;
            events_trait!(#em_tr, #(#(#e_strcts),*),*);
            c_manager_trait!(#cm_tr, #(#c_types),*);
        }
        pub use _engine::{#cm_tr, #em_tr, LabelTrait};
    )
}

#[proc_macro]
pub fn component_manager(_input: TokenStream) -> TokenStream {
    // Get dependencies
    let dir = Span::call_site().unwrap().source_file().path();
    let is_main = dir
        .file_stem()
        .expect(format!("Could not get file stem for: {:#?}", dir).as_str())
        .to_string_lossy()
        == "main";
    let dir = dir
        .ancestors()
        .nth(dir.ancestors().count().max(2) - 2)
        .expect(format!("Could not get topmost directory from path: {:#?}", dir).as_str())
        .to_string_lossy()
        .replace("-", "_")
        .to_string();

    let mut f = Out::new(if is_main { "out3.txt" } else { "out4.txt" }, false);
    f.write("Hi".to_string());

    let code = if is_main {
        main_component_manager(dir)
    } else {
        sub_component_manager(dir)
    };

    // Open out.txt to print stuff
    f.write(format!("Code:\n{}\n", code));

    code.into()
}
