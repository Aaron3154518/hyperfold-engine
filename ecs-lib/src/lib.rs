extern crate proc_macro;
use std::cmp::Ordering;
use std::io::Write;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote, ToTokens};
use regex::Regex;
use syn;
use syn::parse_macro_input;

struct Out {
    f: std::fs::File,
}

impl Out {
    pub fn new() -> Self {
        Self {
            f: std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open("out.txt")
                .expect("Could not open out.txt"),
        }
    }

    pub fn write(&mut self, s: String) {
        self.f
            .write(s.as_bytes())
            .expect("Could not write to out.txt");
    }
}

fn to_path(s: &str, sep: &'static str) -> syn::Path {
    syn::Path {
        leading_colon: None,
        segments: s
            .split(sep)
            .map(|s| syn::PathSegment {
                ident: format_ident!("{}", s),
                arguments: syn::PathArguments::None,
            })
            .collect(),
    }
}

#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut strct = parse_macro_input!(item as syn::ItemStruct);
    strct.vis = syn::parse_quote!(pub);

    quote!(#strct).into()
}

#[proc_macro_attribute]
pub fn system(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut fun = parse_macro_input!(item as syn::ItemFn);

    // // Extract function names
    // let regex = Regex::new(r"\w+(::\w+)*").expect("msg");
    // let s_names = services
    //     .iter()
    //     .map(|s| match regex.find(s) {
    //         Some(m) => m.as_str(),
    //         None => "",
    //     })
    //     .filter(|s| !s.is_empty())
    //     .map(|s| to_path(s, "::"))
    //     .collect::<Vec<_>>();

    // let (s_names, s_arg_vs, s_arg_ts) = (Vec::new(), Vec::new(), Vec::new());
    // for s in services.iter() {
    //     let (a_vec, m_vec, t_vec) = (Vec::new(), Vec::new(), Vec::new());
    //     for m in regex.captures_iter(s) {
    //         match (m.name("arg"), m.name("mut"), m.name("type")) {
    //             (Some(a), Some(m), Some(t)) => {
    //                 let tidx = t.as_str().parse::<usize>().expect("Could not parse type");
    //                 match (c_vars.get(tidx), c_types.get(tidx)) {
    //                     (Some(v), Some(ty)) => {
    //                         a_vec.push(a.as_str());
    //                         m_vec.push(v.to_owned());
    //                         let mutable =
    //                             m.as_str().parse::<u8>().expect("Could not parse mut") != 0;
    //                         t_vec.push(
    //                             syn::parse_str::<syn::Type>(
    //                                 format!(
    //                                     "&{}{}",
    //                                     if mutable { "mut " } else { "" },
    //                                     ty.to_token_stream().to_string()
    //                                 )
    //                                 .as_str(),
    //                             )
    //                             .expect("Stoopid"),
    //                         );
    //                     }
    //                     _ => (),
    //                 }
    //             }
    //             _ => (),
    //         }
    //     }
    //     s_names.push(a_vec);
    //     s_arg_vs.push(value)
    // }

    // Open out.txt to print stuff
    // let mut f = Out::new();
    // f.write(format!("Fn: {:#?}\n", fun));

    quote!(#fun).into()
}

#[derive(Debug)]
struct Input {
    sm: syn::Ident,
    _comma: syn::Token![,],
    cm: syn::Ident,
}

impl Input {
    pub fn get(self) -> (syn::Ident, syn::Ident) {
        (self.sm, self.cm)
    }
}

impl syn::parse::Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            sm: input.parse()?,
            _comma: input.parse()?,
            cm: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn component_manager(input: TokenStream) -> TokenStream {
    // Components
    let components = std::env::var("COMPONENTS")
        .unwrap_or_else(|_| "COMPONENTS".to_string())
        .split(" ")
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    // Parse out the paths and components
    // let regex = Regex::new(r"(\w+)(?:::|$)").expect("msg");
    // let mods = components
    //     .iter()
    //     .map(|s| {
    //         regex
    //             .captures_iter(s)
    //             .map(|p| p.get(1).expect("Uh Oh").as_str().to_string())
    //             .collect::<Vec<_>>()
    //     })
    //     .collect::<Vec<_>>();

    // Contruct types
    let c_types = components
        .iter()
        .map(|t| syn::parse_str::<syn::Type>(t).expect("Stoopid"))
        .collect::<Vec<_>>();

    // Construct variable names
    let c_vars = c_types
        .iter()
        .enumerate()
        .map(|(i, _t)| format_ident!("c{}", i))
        .collect::<Vec<_>>();

    // Services
    let services = std::env::var("SERVICES")
        .unwrap_or_else(|_| "SERVICES".to_string())
        .split(" ")
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    // Split function into args
    let regex = Regex::new(r"(?P<arg>\w+):(?P<mut>[01]):(?P<type>\d+)(,|\)$)").expect("msg");
    let mut s_arg_types = services
        .iter()
        .map(|s| {
            let mut types = regex
                .captures_iter(s)
                .filter_map(|m| match (m.name("mut"), m.name("type")) {
                    (Some(m), Some(t)) => Some((
                        m.as_str().parse::<u8>().expect("Could not parse mut") != 0,
                        t.as_str().parse::<usize>().expect("Could not parse type"),
                    )),
                    _ => None,
                })
                .collect::<Vec<_>>();
            types.sort_by(|arg1, arg2| arg1.1.partial_cmp(&arg2.1).unwrap());
            types
        })
        .collect::<Vec<_>>();
    // Sort
    s_arg_types.sort_by(|args1, args2| {
        // Sort length first
        if args1.len() < args2.len() {
            Ordering::Less
        } else if args1.len() > args2.len() {
            Ordering::Greater
        } else {
            // Sort by type indices
            for (a1, a2) in args1.iter().zip(args2.iter()) {
                if a1.1 < a2.1 {
                    return Ordering::Less;
                } else if a1.1 > a2.1 {
                    return Ordering::Greater;
                }
            }
            // Sort by mutability
            for (a1, a2) in args1.iter().zip(args2.iter()) {
                if !a1.0 && a2.0 {
                    return Ordering::Less;
                } else if a1.0 && !a2.0 {
                    return Ordering::Greater;
                }
            }
            Ordering::Equal
        }
    });
    // Remove duplicate signatures
    s_arg_types.dedup();

    // Convert to types
    let (s_arg_vs, s_arg_ts) = s_arg_types
        .iter()
        .map(|args| {
            args.iter()
                .map(|arg| {
                    (
                        format_ident!("c{}", arg.1),
                        syn::parse_str::<syn::Type>(
                            format!(
                                "&{}{}",
                                if arg.0 { "mut " } else { "" },
                                c_types
                                    .get(arg.1)
                                    .expect(format!("Invalid component index: {}", arg.1).as_str())
                                    .to_token_stream()
                                    .to_string()
                            )
                            .as_str(),
                        )
                        .expect("Could not parse type"),
                    )
                })
                .unzip::<_, _, Vec<_>, Vec<_>>()
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    // let (s_arg_vs, s_arg_ts) = services
    //     .iter()
    //     .map(|s| {
    //         regex
    //             .captures_iter(s)
    //             .filter_map(|m| match (m.name("arg"), m.name("mut"), m.name("type")) {
    //                 (Some(_a), Some(m), Some(t)) => {
    //                     let tidx = t.as_str().parse::<usize>().expect("Could not parse type");
    //                     match (c_vars.get(tidx), c_types.get(tidx)) {
    //                         (Some(v), Some(ty)) => {
    //                             let mutable =
    //                                 m.as_str().parse::<u8>().expect("Could not parse mut") != 0;
    //                             Some((
    //                                 v.to_owned(),
    //                                 syn::parse_str::<syn::Type>(
    //                                     format!(
    //                                         "&{}{}",
    //                                         if mutable { "mut " } else { "" },
    //                                         ty.to_token_stream().to_string()
    //                                     )
    //                                     .as_str(),
    //                                 )
    //                                 .expect("Stoopid"),
    //                             ))
    //                         }
    //                         _ => None,
    //                     }
    //                 }
    //                 _ => None,
    //             })
    //             .unzip::<_, _, Vec<_>, Vec<_>>()
    //     })
    //     .unzip::<_, _, Vec<_>, Vec<_>>();

    let (sm, cm) = parse_macro_input!(input as Input).get();

    let code = quote!(
        use ecs_macros::*;
        manager!(#cm, #(#c_vars, #c_types),*);
        systems!(#sm, #cm, #((#(#s_arg_vs, #s_arg_ts),*)),*);
    );

    // Open out.txt to print stuff
    let mut f = Out::new();
    f.write(format!("Components:\n{:#?}\n", components));
    f.write(format!("Services:\n{:#?}\n", services));
    f.write(format!("Code:\n{}\n", code));

    code.into()
}
