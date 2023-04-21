#![feature(proc_macro_span)]
#![feature(slice_group_by)]

extern crate proc_macro;
use std::cmp::Ordering;
use std::io::Write;
use std::path::PathBuf;
use std::process::CommandArgs;
use std::str::FromStr;

use ecs_macros::structs::ComponentArgs;
use num_traits::FromPrimitive;
use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use regex::Regex;
use syn;
use syn::parse_macro_input;

fn partition<T, U, F>(mut v: Vec<(T, U)>, f: F) -> Vec<(Vec<T>, U)>
where
    T: Clone,
    U: Ord + Eq + Clone,
    F: FnMut(&(T, U), &(T, U)) -> Ordering,
{
    v.sort_by(f);
    v.group_by(|v1, v2| v1.1 == v2.1)
        .map(|s| s.to_vec().into_iter().unzip::<_, _, Vec<_>, Vec<_>>())
        .filter_map(|v| match v.1.into_iter().next() {
            Some(u) => Some((v.0, u)),
            None => None,
        })
        .collect::<Vec<_>>()
}

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

#[derive(Debug)]
struct ComponentInput {
    args: Vec<String>,
}

impl ComponentInput {
    pub fn get(self) -> Vec<String> {
        self.args
    }
}

impl syn::parse::Parse for ComponentInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = Vec::new();
        loop {
            match input.parse::<syn::Ident>() {
                Ok(i) => args.push(i.to_string()),
                Err(_) => break,
            };
            if let Err(_) = input.parse::<syn::Token![,]>() {
                break;
            }
        }
        Ok(Self { args })
    }
}

#[proc_macro_attribute]
pub fn component(input: TokenStream, item: TokenStream) -> TokenStream {
    let mut out = Out::new("out.txt", true);

    let args = parse_macro_input!(input as ComponentInput).get();
    out.write(format!("{:#?}\n", args));

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
    let en = parse_macro_input!(item as syn::ItemEnum);

    let mut f = Out::new("out_en.txt", false);

    let code = quote!(
        #en
    );
    f.write(format!("{:#?}\n", code));

    code.into()
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
    let mut f = Out::new("out3.txt", false);

    // Components
    let components = std::env::var("COMPONENTS")
        .unwrap_or_else(|_| "COMPONENTS".to_string())
        .split(" ")
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    // Extract component names, types, and args
    let r =
        Regex::new(r"(?P<name>\w+(::\w+)*)\((?P<args>\d+)\)").expect("Could not construct regex");
    let comps = components
        .iter()
        .filter_map(|s| {
            if let Some(c) = r.captures(s) {
                if let (Some(name), Some(args)) = (c.name("name"), c.name("args")) {
                    if let Some(a) = <ComponentArgs as FromPrimitive>::from_u8(
                        args.as_str()
                            .parse::<u8>()
                            .expect("Could not parse component type code"),
                    ) {
                        return Some((name.as_str().to_string(), a));
                    }
                }
            }
            None
        })
        .enumerate()
        .map(|(i, (s, t))| {
            (
                (
                    format_ident!("c{}", i),
                    syn::parse_str::<syn::Type>(s.as_str())
                        .expect(format!("Could not parse Component type: {:#?}", t).as_str()),
                ),
                t,
            )
        })
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
    let s_args = services
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
            types.1.sort_by(|arg1, arg2| {
                arg1.1
                    .partial_cmp(&arg2.1)
                    .expect(format!("Could not compare {} and {}", arg1.1, arg2.1).as_str())
            });
            types
        })
        .collect::<Vec<_>>();
    // Partition services by signature
    let (s_names, s_args) = partition(s_args, |args1, args2| {
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
    })
    .into_iter()
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

    // Construct argument types from signatures
    let s_args = s_args
        .iter()
        .map(|s| {
            s.iter()
                .map(|(m, a)| {
                    let comp = comps
                        .get(*a)
                        .expect(format!("Invalid component index: {}", a).as_str());
                    (
                        (
                            comp.0 .0.to_owned(),
                            syn::parse_str::<syn::Type>(
                                format!(
                                    "&{}{}",
                                    if *m { "mut " } else { "" },
                                    comp.0 .1.to_token_stream().to_string()
                                )
                                .as_str(),
                            )
                            .expect("Could not parse type"),
                        ),
                        comp.1,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let (mut s_carg_vs, mut s_carg_ts, mut s_garg_vs, mut s_garg_ts) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    s_args.into_iter().for_each(|v| {
        s_carg_vs.push(Vec::new());
        s_carg_ts.push(Vec::new());
        s_garg_vs.push(Vec::new());
        s_garg_ts.push(Vec::new());
        partition(v, |c1, c2| c1.1.cmp(&c2.1))
            .into_iter()
            .for_each(|(c, a)| {
                let (vars, types) = c.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();
                let (vs, ts) = match a {
                    ComponentArgs::None => (&mut s_carg_vs, &mut s_carg_ts),
                    ComponentArgs::Global => (&mut s_garg_vs, &mut s_garg_ts),
                };
                vs.pop();
                vs.push(vars);
                ts.pop();
                ts.push(types);
            })
    });

    // Partition components into types
    let (mut c_vars, mut c_types, mut g_vars, mut g_types) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    partition(comps, |c1, c2| c1.1.cmp(&c2.1))
        .into_iter()
        .for_each(|(c, a)| {
            let c = c.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();
            match a {
                ComponentArgs::None => (c_vars, c_types) = c,
                ComponentArgs::Global => (g_vars, g_types) = c,
            }
        });

    f.write(format!(
        "{:#?}\n{:#?}\n{:#?}\n{:#?}\n",
        s_carg_vs, s_carg_ts, s_garg_vs, s_garg_ts
    ));

    let (sm, cm) = parse_macro_input!(input as Input).get();

    let code = quote!(
        use ecs_macros::*;
        manager!(#cm, c(#(#c_vars, #c_types),*), g(#(#g_vars, #g_types),*));
        systems!(#sm, #cm,
            #(
                ((#(#s_names),*),
                c(#(#s_carg_vs, #s_carg_ts),*),
                g(#(#s_garg_vs, #s_garg_ts),*))
            ),*
        );
    );

    // Open out.txt to print stuff
    // f.write(format!("Args:\n{:#?}\n", s_arg_ts));
    // f.write(format!("Components:\n{:#?}\n", components));
    // f.write(format!("Services:\n{:#?}\n", services));
    f.write(format!("Code:\n{}\n", code));

    code.into()
}
