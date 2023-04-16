extern crate proc_macro;
use std::io::Write;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
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

#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut strct = parse_macro_input!(item as syn::ItemStruct);
    strct.vis = syn::parse_quote!(pub);

    quote!(#strct).into()
}

#[proc_macro_attribute]
pub fn system(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut fun = parse_macro_input!(item as syn::ItemFn);

    // Open out.txt to print stuff
    let mut f = Out::new();
    // f.write(format!("Fn: {:#?}\n", fun));

    // Get the function name and visibility
    let name = &fun.sig.ident;
    let vis = &fun.vis;

    // Extract the argument types and their names
    let args = fun
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Typed(arg) => Some((arg.ty.clone(), quote!(#arg).to_string())),
            _ => None,
        })
        .map(|(ty, name)| {
            let module_path = match *ty {
                syn::Type::Path(type_path) => {
                    let mut segments = type_path.path.segments.iter();
                    if let Some(first_segment) = segments.next() {
                        let mut resolved = first_segment.clone();
                        let mut path = first_segment.ident.to_string();
                        // for segment in segments {
                        //     path.push_str("::");
                        //     path.push_str(&segment.ident.to_string());
                        //     if let Some(res) = resolved
                        //         .arguments
                        //         .as_mut()
                        //         .map(|args| args.as_mut().iter_mut().next())
                        //         .flatten()
                        //     {
                        //         *res = PathArguments::None;
                        //     }
                        //     resolved.ident = segment.ident.clone();
                        //     resolved.arguments = None;
                        //     if let Ok((_, res)) = type_path
                        //         .path
                        //         .clone()
                        //         .resolved_at(resolved.span(), quote!(self).into())
                        //     {
                        //         resolved = res;
                        //     } else {
                        //         break;
                        //     }
                        // }
                        Some(quote!(Some(#path.to_string())))
                    } else {
                        None
                    }
                }
                _ => None,
            };
        });

    quote!(#fun).into()
}

#[derive(Debug)]
struct Module {
    path: Vec<String>,
    components: Vec<String>,
}

impl Module {
    pub fn new(path: Vec<String>, components: Vec<String>) -> Self {
        Self {
            path: path,
            components,
        }
    }

    pub fn get_types(&self) -> Vec<String> {
        self.components
            .iter()
            .map(|c| format!("{}::{}", self.path.join("::"), c))
            .collect()
    }
}

#[proc_macro]
pub fn component_manager(item: TokenStream) -> TokenStream {
    // Get the data
    let data = std::env::var("COMPONENTS").unwrap_or_else(|_| "COMPONENTS".to_string());

    // Parse out the paths and components
    let path_r = Regex::new(r"(\w+)::").expect("msg");
    let comps_r = Regex::new(r"(\w+)[,\}]").expect("msg");
    let mods = data
        .split(" ")
        .map(|s| {
            Module::new(
                path_r
                    .captures_iter(s)
                    .map(|p| p.get(1).expect("Uh oh").as_str().to_string())
                    .collect(),
                comps_r
                    .captures_iter(s)
                    .map(|p| p.get(1).expect("Uh oh").as_str().to_string())
                    .collect(),
            )
        })
        .filter(|m| !m.components.is_empty())
        .collect::<Vec<_>>();

    // Contruct types
    let mut types = Vec::new();
    for m in mods {
        types.append(&mut m.get_types());
    }
    let types = types
        .iter()
        .map(|t| syn::parse_str::<syn::Type>(t).expect("Stoopid"))
        .collect::<Vec<_>>();

    // Construct variable names
    let vars = types
        .iter()
        .enumerate()
        .map(|(i, _t)| format_ident!("c{}", i))
        .collect::<Vec<_>>();

    let manager = parse_macro_input!(item as syn::Ident);

    let code = quote!(
        use ecs_macros::{manager, ComponentManager};
        manager!(#manager, #(#vars, #types),*);
    );

    // Open out.txt to print stuff
    let mut f = Out::new();
    f.write(format!("Data: {}\n", data));
    f.write(format!("Code:\n{}\n", code));

    code.into()
}
