use std::{env, path::PathBuf};

// use input::{Input, Input2};
use parser::{codegen::codegen::Decoder, util::format_code};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use shared::{
    file::Out,
    parse_args::{ComponentMacroArgs, GlobalMacroArgs},
};
use syn::{parse_macro_input, parse_quote};

// mod input;

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
    let decoder = Decoder::new();
    let (cr_idx, code) = decoder.codegen(PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("No manifest directory specified"),
    ));

    if cr_idx == 0 {
        let mut f = Out::new("out2.rs", false);
        f.write(format!("{}\n", format_code(code.to_string())));
    }

    code.into()
}

// #[proc_macro]
// pub fn query(input: TokenStream) -> TokenStream {
//     let Input {
//         func,
//         body,
//         query,
//         event,
//         comp_vars,
//         comp_mut,
//         comp_types,
//         glob_vars,
//         glob_mut,
//         glob_types,
//     } = parse_macro_input!(input as Input);

//     let ev = format_ident!("event");
//     let eid = format_ident!("eid");

//     quote!(
//         fn #func(#query { event: #ev, eid: #eid, components: (#(#comp_vars),*), globals: (#(#glob_vars),*), .. } :
//             #query<#event, (#(&#comp_mut #comp_types),*), (#(&#glob_mut #glob_types),*)>) {
//                 #body
//             }
//     )
//     .into()
// }

// #[proc_macro]
// pub fn query2(input: TokenStream) -> TokenStream {
//     let Input2 {
//         ident,
//         vars,
//         muts,
//         types,
//     } = parse_macro_input!(input as Input2);

//     quote!(
//         pub struct #ident<'a> {
//             #(#vars: &'a #muts #types),*
//         }
//     )
//     .into()
// }