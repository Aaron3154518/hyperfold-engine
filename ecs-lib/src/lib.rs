extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn add_hello_world(_: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input tokens into an AST
    let input_fn = parse_macro_input!(input as ItemFn);

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
