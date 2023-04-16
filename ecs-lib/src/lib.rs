extern crate proc_macro;
use std::io::Write;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use regex::Regex;
use syn;
use syn::parse_macro_input;
use syn::punctuated::Punctuated;

macro_rules! path_segment {
    ($s: ident) => {
        syn::PathSegment::from(format_ident!("{}", $s))
    };

    ($s: literal) => {
        syn::PathSegment::from(format_ident!("{}", $s))
    };
}

fn copy_push_segment(
    segments: &syn::punctuated::Punctuated<syn::PathSegment, syn::token::PathSep>,
    name: &String,
) -> syn::punctuated::Punctuated<syn::PathSegment, syn::token::PathSep> {
    let mut segments = segments.clone();
    segments.push(path_segment!(name));
    segments
}

#[proc_macro_attribute]
pub fn add_hello_world(_attr: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input tokens into an AST
    let input_fn = parse_macro_input!(input as syn::ItemFn);

    // Extract the name of the function
    let fn_name = input_fn.sig.ident;

    // Extract the body of the function
    let fn_body = input_fn.block;

    // Create a new block that adds the `println!("Hello World");` statement
    let new_fn_body = quote! {
        {
            #fn_body
            println!("Hello World");
        }
    };

    // Generate the new function definition with the modified body
    let output = quote! {
        fn #fn_name() #new_fn_body
    };

    // Parse the generated output tokens back into a TokenStream and return it
    output.into()
}

#[proc_macro_attribute]
pub fn make_foo(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input function
    let input = parse_macro_input!(item as syn::ItemFn);

    // Get the name of the function
    let name = &input.sig.ident;

    let t = syn::Ident::new(attr.to_string().as_str(), Span::call_site());
    let strct_fun = syn::Ident::new(format!("call_{}", name).as_str(), Span::call_site());

    // Generate a struct with a `Foo` method that calls the input function
    let output = quote! {
        #[add_hello_world]
        #input

        impl #t {
            pub fn #strct_fun(&self) {
                #name();
            }
        }
    };

    // Return the generated code as a TokenStream
    output.into()
}

#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut strct = parse_macro_input!(item as syn::ItemStruct);
    strct.vis = syn::parse_quote!(pub);

    quote!(#strct).into()
}

#[derive(Debug)]
struct Module {
    path: Vec<String>,
    components: Vec<String>,
}

impl Module {
    pub fn new(path: Vec<String>, components: Vec<String>) -> Self {
        Self {
            // Remove main
            path: {
                let mut v = if path.len() == 1
                    && match path.first() {
                        Some(p) => p == "main",
                        None => false,
                    } {
                    Vec::new()
                } else {
                    path
                };
                v.insert(0, "crate".to_string());
                v
            },
            components,
        }
    }

    pub fn to_path(&self) -> Vec<syn::Path> {
        let segments = self
            .path
            .iter()
            .map(|p| path_segment!(p))
            .collect::<Punctuated<_, syn::token::PathSep>>();
        self.components
            .iter()
            .map(|c| syn::Path {
                leading_colon: None,
                segments: copy_push_segment(&segments, c),
            })
            .collect::<Vec<_>>()
    }

    pub fn get_vec_types(&self) -> Vec<String> {
        self.components
            .iter()
            .map(|c| format!("Vec<{}::{}>", self.path.join("::"), c))
            .collect()
    }
}

#[proc_macro_attribute]
pub fn component_manager(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("out.txt")
        .expect("Nope");

    let data = std::env::var("COMPONENTS").unwrap_or_else(|_| "COMPONENTS".to_string());
    f.write(format!("Data: {}\n", data).as_bytes())
        .expect("Nope");

    let path_r = Regex::new(r"(\w+)::").expect("msg");
    let comps_r = Regex::new(r"(\w+)[,\}]").expect("msg");
    let mut mods = data
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

    let mut types = Vec::new();
    for m in mods {
        types.append(&mut m.get_vec_types());
    }

    let vars = types
        .iter()
        .enumerate()
        .map(|(i, _t)| format_ident!("c{}", i))
        .collect::<Vec<_>>();

    let mut manager = parse_macro_input!(item as syn::ItemStruct);

    // Overwrite fields
    if let syn::Fields::Named(named_fields) = &mut manager.fields {
        named_fields.named = vars
            .iter()
            .zip(types.iter())
            .map(|(v, t)| syn::Field {
                attrs: Vec::new(),
                vis: syn::Visibility::Inherited,
                mutability: syn::FieldMutability::None,
                ident: Some(v.clone()),
                colon_token: Some(syn::token::Colon {
                    spans: [proc_macro2::Span::call_site()],
                }),
                ty: syn::parse_str(&t).expect("Stoopid"),
            })
            .collect();
    }

    let manager_ident = manager.ident.clone();

    // TODO: relative to this
    let code = quote!(
        #manager

        impl #manager_ident {
            pub fn new() -> Self {
                Self {
                    #(#vars: Vec::new()),*
                }
            }
        }
    );

    f.write(format!("Code:\n{}\n", code).as_bytes())
        .expect("msg");

    code.into()
}
