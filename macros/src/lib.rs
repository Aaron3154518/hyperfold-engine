use std::{
    env::{self, temp_dir},
    fs,
    path::PathBuf,
};

use parser::codegen::ast_codegen::{INDEX, INDEX_SEP};
use proc_macro::TokenStream;
use quote::quote;
use shared::{
    parse_args::{ComponentMacroArgs, GlobalMacroArgs},
    util::Catch,
};
use syn::{parse_macro_input, parse_quote};

#[proc_macro_attribute]
pub fn component(input: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as ComponentMacroArgs);
    if args.is_dummy {
        return quote!().into();
    }

    let mut input = parse_macro_input!(item as syn::ItemStruct);
    input.vis = syn::parse_quote!(pub);

    quote!(#input).into()
}

#[proc_macro_attribute]
pub fn global(input: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as GlobalMacroArgs);
    if args.is_dummy {
        return quote!().into();
    }

    let mut input = parse_macro_input!(item as syn::ItemStruct);
    input.vis = syn::parse_quote!(pub);

    quote!(#input).into()
}

#[proc_macro_attribute]
pub fn system(_input: TokenStream, item: TokenStream) -> TokenStream {
    let mut fun = parse_macro_input!(item as syn::ItemFn);
    fun.vis = parse_quote!(pub);

    quote!(#fun).into()
}

#[proc_macro_attribute]
pub fn event(_input: TokenStream, item: TokenStream) -> TokenStream {
    let mut ev = parse_macro_input!(item as syn::ItemStruct);
    ev.vis = parse_quote!(pub);

    quote!(#ev).into()
}

#[proc_macro]
pub fn game_crate(_input: TokenStream) -> TokenStream {
    let dir = fs::canonicalize(PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("No manifest directory specified"),
    ))
    .expect("Could not canonicalize manifest directory");

    let data = fs::read_to_string(temp_dir().join(INDEX)).expect("Could not read index file");
    let file = data
        .split("\n")
        .find_map(|line| {
            line.split_once(INDEX_SEP)
                .and_then(|(path, file)| (dir == PathBuf::from(path)).then_some(file))
        })
        .catch(format!(
            "Could not find directory in index: {}",
            dir.display()
        ));

    quote!(include!(#file);).into()
}
