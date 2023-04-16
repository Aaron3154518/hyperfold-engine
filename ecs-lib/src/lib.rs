extern crate proc_macro;
use std::io::Write;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use regex::Regex;
use syn;
use syn::parse_macro_input;

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

    pub fn get_types(&self) -> Vec<String> {
        self.components
            .iter()
            .map(|c| format!("{}::{}", self.path.join("::"), c))
            .collect()
    }
}

#[proc_macro]
pub fn component_manager(item: TokenStream) -> TokenStream {
    // Open out.txt to print stuff
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("out.txt")
        .expect("Nope");

    // Get the data
    let data = std::env::var("COMPONENTS").unwrap_or_else(|_| "COMPONENTS".to_string());
    f.write(format!("Data: {}\n", data).as_bytes())
        .expect("Nope");

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

    // TODO: relative to this
    let code = quote!(
        use ecs_macros::{manager, ComponentManager};
        manager!(#manager, #(#vars, #types),*);
    );

    f.write(format!("Code:\n{}\n", code).as_bytes())
        .expect("msg");

    code.into()
}
