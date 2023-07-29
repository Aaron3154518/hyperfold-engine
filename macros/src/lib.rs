use std::{
    env::{self, temp_dir},
    fs,
    path::PathBuf,
};

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use shared::{
    constants::{INDEX, INDEX_SEP, STATE_DATA, STATE_ENTER_EVENT, STATE_EXIT_EVENT, STATE_LABEL},
    parsing::{ComponentMacroArgs, GlobalMacroArgs},
    traits::Catch,
};
use syn::{parse_macro_input, parse_quote};

#[proc_macro_attribute]
pub fn component(input: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as ComponentMacroArgs);
    if args.is_dummy {
        return quote!().into();
    }

    let mut input = parse_macro_input!(item as syn::Item);
    match &mut input {
        syn::Item::Struct(syn::ItemStruct { vis, .. })
        | syn::Item::Enum(syn::ItemEnum { vis, .. }) => *vis = syn::parse_quote!(pub),
        _ => panic!("Invalid component item: {input:#?}"),
    };

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

#[proc_macro_attribute]
pub fn state(_input: TokenStream, item: TokenStream) -> TokenStream {
    let mut data_strct = parse_macro_input!(item as syn::ItemStruct);

    data_strct.vis = parse_quote!(pub);
    let name = std::mem::replace(&mut data_strct.ident, format_ident!("{STATE_DATA}"));

    let enter_event = format_ident!("{STATE_ENTER_EVENT}");
    let exit_event = format_ident!("{STATE_EXIT_EVENT}");
    let label = format_ident!("{STATE_LABEL}");

    quote!(
        #[allow(non_snake_case)]
        pub mod #name {
            #[warn(non_snake_case)]
            #data_strct
            pub struct #enter_event;
            pub struct #exit_event;
            pub struct #label;
        }
    )
    .into()
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
