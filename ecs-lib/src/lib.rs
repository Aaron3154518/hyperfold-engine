#![feature(proc_macro_span)]
#![feature(slice_group_by)]

extern crate proc_macro;
use std::cmp::Ordering;
use std::io::Write;
use std::path::PathBuf;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::spanned::Spanned;
use quote::{format_ident, quote, ToTokens};
use regex::Regex;
use syn;
use syn::parse_macro_input;

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

    // Get service functions
    let services = std::env::var("SERVICES")
        .unwrap_or_else(|_| "SERVICES".to_string())
        .split(" ")
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    // Extract function paths
    let regex = Regex::new(r"\w+(::\w+)*").expect("msg");
    let mut s_names = services
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
    let path_r = Regex::new(r"(?P<path>\w+)(::|\()").expect("Could not construct regex");
    let args_r = Regex::new(r"(?P<arg>\w+):(?P<mut>[01]):(?P<type>\d+)(,|\)$)").expect("msg");
    let mut s_args = services
        .iter()
        .map(|s| {
            let mut types = (
                path_r
                    .captures_iter(s)
                    .filter_map(|c| match c.name("path") {
                        Some(p) => Some(format_ident!("{}", p.as_str())),
                        None => None,
                    })
                    .collect::<Vec<_>>(),
                args_r
                    .captures_iter(s)
                    .filter_map(|c| match (c.name("mut"), c.name("type")) {
                        (Some(m), Some(t)) => Some((
                            m.as_str().parse::<u8>().expect("Could not parse mut") != 0,
                            t.as_str().parse::<usize>().expect("Could not parse type"),
                        )),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            );
            types
                .1
                .sort_by(|arg1, arg2| arg1.1.partial_cmp(&arg2.1).unwrap());
            types
        })
        .collect::<Vec<_>>();
    // Sort
    s_args.sort_by(|args1, args2| {
        // Sort length first
        if args1.1.len() < args2.1.len() {
            Ordering::Less
        } else if args1.1.len() > args2.1.len() {
            Ordering::Greater
        } else {
            // Sort by type indices
            for (a1, a2) in args1.1.iter().zip(args2.1.iter()) {
                if a1.1 < a2.1 {
                    return Ordering::Less;
                } else if a1.1 > a2.1 {
                    return Ordering::Greater;
                }
            }
            // Sort by mutability
            for (a1, a2) in args1.1.iter().zip(args2.1.iter()) {
                if !a1.0 && a2.0 {
                    return Ordering::Less;
                } else if a1.0 && !a2.0 {
                    return Ordering::Greater;
                }
            }
            Ordering::Equal
        }
    });

    // Combine duplicate signatures
    let (s_names, s_args) = s_args
        .group_by(|s1, s2| s1.1 == s2.1)
        .map(|s| s.to_vec().into_iter().unzip::<_, _, Vec<_>, Vec<_>>())
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let s_args = s_args
        .into_iter()
        .filter_map(|v| v.into_iter().next())
        .collect::<Vec<_>>();

    // Construct out argument variable names and types
    let (s_arg_vs, s_arg_ts) = s_args
        .iter()
        .map(|s| {
            s.iter()
                .map(|(m, a)| {
                    (
                        format_ident!("c{}", a),
                        syn::parse_str::<syn::Type>(
                            format!(
                                "&{}{}",
                                if *m { "mut " } else { "" },
                                c_types
                                    .get(*a)
                                    .expect(format!("Invalid component index: {}", a).as_str())
                                    .to_token_stream()
                                    .to_string()
                            )
                            .as_str(),
                        )
                        .expect("Could no parse type"),
                    )
                })
                .unzip::<_, _, Vec<_>, Vec<_>>()
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    // Convert function name path to path
    let s_names = s_names
        .into_iter()
        .map(|names| {
            names
                .into_iter()
                .map(|n| syn::Path {
                    leading_colon: None,
                    segments: n
                        .into_iter()
                        .map(|i| syn::PathSegment {
                            ident: i,
                            arguments: syn::PathArguments::None,
                        })
                        .collect(),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let (sm, cm) = parse_macro_input!(input as Input).get();

    let code = quote!(
        use ecs_macros::*;
        manager!(#cm, #(#c_vars, #c_types),*);
        systems!(#sm, #cm, #(((#(#s_names),*), #(#s_arg_vs, #s_arg_ts),*)),*);
    );

    // Open out.txt to print stuff
    let mut f = Out::new("out3.txt", false);
    // f.write(format!("Args:\n{:#?}\n", s_arg_ts));
    // f.write(format!("Components:\n{:#?}\n", components));
    // f.write(format!("Services:\n{:#?}\n", services));
    f.write(format!("Code:\n{}\n", code));

    code.into()
}
