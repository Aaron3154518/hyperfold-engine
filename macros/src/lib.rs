use std::{
    env::{self, temp_dir},
    fs,
    path::PathBuf,
};

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use shared::{
    constants::{INDEX, INDEX_SEP, STATE_DATA, STATE_ENTER_EVENT, STATE_EXIT_EVENT, STATE_LABEL},
    msg_result::CombineMsgs,
    parsing::{ComponentMacroArgs, GlobalMacroArgs, SystemMacroArgs},
    syn::{parse_tokens, vec_to_path, Parse, ParseMsg, ToCompileErr},
    traits::{Catch, CollectVecInto},
};
use syn::{parse_macro_input, parse_quote, spanned::Spanned};

macro_rules! parse_macro_input2 {
    ($v: ident as $ty: ty) => {
        match syn::parse::<$ty>($v) {
            Ok(t) => t,
            Err(err) => {
                return err.to_compile_error();
            }
        }
    };
}

fn parse_and_quote<T>(
    input: TokenStream,
    f: impl FnOnce(T, Span) -> proc_macro2::TokenStream,
) -> TokenStream
where
    T: Parse,
{
    let input: proc_macro2::TokenStream = input.into();
    let input_span = input.span();
    match parse_tokens(input) {
        Ok(t) => f(t, input_span),
        Err(errs) => {
            let errs = errs.map_vec_into(|msg| {
                let (msg, span) = match msg {
                    ParseMsg::Diagnostic { msg, span } => (msg, span),
                    ParseMsg::String(msg) => (msg, input_span),
                };
                syn::Error::new(span, msg).into_compile_error()
            });
            quote!(#(#errs)*)
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn component(input: TokenStream, item: TokenStream) -> TokenStream {
    parse_and_quote(input, |args: ComponentMacroArgs, _| match args.is_dummy {
        true => quote!(),
        false => {
            let mut item = parse_macro_input2!(item as syn::Item);
            match &mut item {
                syn::Item::Struct(syn::ItemStruct { vis, .. })
                | syn::Item::Enum(syn::ItemEnum { vis, .. }) => *vis = syn::parse_quote!(pub),
                _ => {
                    return syn::Error::new(item.span(), "Invalid component item")
                        .into_compile_error()
                }
            };
            quote!(#item)
        }
    })
}

#[proc_macro_attribute]
pub fn global(input: TokenStream, item: TokenStream) -> TokenStream {
    parse_and_quote(input, |args: GlobalMacroArgs, _| match args.is_dummy {
        true => quote!(),
        false => {
            let mut strct = parse_macro_input2!(item as syn::ItemStruct);
            strct.vis = syn::parse_quote!(pub);

            quote!(#strct)
        }
    })
}

#[proc_macro_attribute]
pub fn system(input: TokenStream, item: TokenStream) -> TokenStream {
    let mut fun = parse_macro_input!(item as syn::ItemFn);
    fun.vis = parse_quote!(pub);
    let input: proc_macro2::TokenStream = input.into();

    quote!(
        crate::system_macro! { #input }
        #fun
    )
    .into()
    // parse_and_quote(input, |args: SystemMacroArgs, span| {
    //     let mut fun = parse_macro_input2!(item as syn::ItemFn);
    //     fun.vis = parse_quote!(pub);

    //     let paths = match match args {
    //         SystemMacroArgs::Init() => Vec::new(),
    //         SystemMacroArgs::System { states } => states.map_vec_into(|(p, _)| vec_to_path(p)),
    //     }
    //     .combine_results()
    //     .to_compile_errors(span)
    //     {
    //         Ok(paths) => paths,
    //         Err(errs) => return errs,
    //     };

    //     quote!(
    //         #fun

    //         #(const _: std::marker::PhantomData<#paths> = std::marker::PhantomData;)*
    //     )
    // })
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
