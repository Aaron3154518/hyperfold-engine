extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
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
            println!("Hello World!");
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
    item
}

#[proc_macro_attribute]
pub fn component_manager(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
