use std::{
    env::{self, temp_dir},
    fs,
    path::PathBuf,
};

use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use shared::{
    constants::{INDEX, INDEX_SEP, STATE_DATA, STATE_ENTER_EVENT, STATE_EXIT_EVENT, STATE_LABEL},
    parsing::{ComponentMacroArgs, GlobalMacroArgs},
    syn::parse_tokens,
};
use syn::{
    parse::Parse, parse_macro_input, parse_quote, spanned::Spanned, Item, ItemEnum, ItemStruct,
};

type Span2 = proc_macro2::Span;
type TokenStream2 = proc_macro2::TokenStream;

macro_rules! error {
    ($span: ident, $msg: expr) => {
        syn::Error::new($span.span(), $msg)
            .into_compile_error()
            .into()
    };
}

macro_rules! try_catch {
    ($span: ident, $expr: expr, $msg: expr) => {
        match $expr {
            Ok(t) => t,
            Err(_) => return error!($span, $msg),
        }
    };
}

// Used to capture the span of an empty TokenStream
struct Empty(Span2);

impl Empty {
    fn span(&self) -> Span2 {
        self.0
    }
}

impl Parse for Empty {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Empty(input.span()))
    }
}

// Represents a union of ItemStruct and ItemEnum
enum StructEnum {
    Struct(ItemStruct),
    Enum(ItemEnum),
}

impl StructEnum {
    fn public(mut self) -> Self {
        match &mut self {
            StructEnum::Struct(ItemStruct { vis, .. }) | StructEnum::Enum(ItemEnum { vis, .. }) => {
                *vis = parse_quote!(pub)
            }
        }
        self
    }

    fn swap_name(mut self, src: impl std::fmt::Display) -> (syn::Ident, Self) {
        match &mut self {
            StructEnum::Struct(ItemStruct { ident, .. })
            | StructEnum::Enum(ItemEnum { ident, .. }) => {
                (std::mem::replace(ident, format_ident!("{src}")), self)
            }
        }
    }

    fn quote(self) -> TokenStream2 {
        quote!(#self)
    }
}

impl ToTokens for StructEnum {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            StructEnum::Struct(s) => tokens.append_all([s]),
            StructEnum::Enum(e) => tokens.append_all([e]),
        }
    }
}

macro_rules! parse_struct_or_enum {
    ($item: ident, $what: literal) => {{
        let item = parse_macro_input!($item as Item);
        match item {
            Item::Struct(s) => StructEnum::Struct(s),
            Item::Enum(e) => StructEnum::Enum(e),
            _ => return error!(item, concat!("Only Structs and Enums may be ", $what)),
        }
    }};
}

#[proc_macro_attribute]
pub fn component(input: TokenStream, item: TokenStream) -> TokenStream {
    match parse_tokens::<ComponentMacroArgs>(input.into()) {
        Ok(args) if !args.is_dummy => parse_struct_or_enum!(item, "Components").public().quote(),
        _ => quote!(),
    }
    .into()
}

#[proc_macro_attribute]
pub fn global(input: TokenStream, item: TokenStream) -> TokenStream {
    match parse_tokens::<GlobalMacroArgs>(input.into()) {
        Ok(args) if !args.is_dummy => parse_struct_or_enum!(item, "Globals").public().quote(),
        _ => quote!(),
    }
    .into()
}

#[proc_macro_attribute]
pub fn event(_input: TokenStream, item: TokenStream) -> TokenStream {
    parse_struct_or_enum!(item, "Events")
        .public()
        .quote()
        .into()
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
}

#[proc_macro_attribute]
pub fn state(_input: TokenStream, item: TokenStream) -> TokenStream {
    let (name, data_struct) = parse_struct_or_enum!(item, "States")
        .public()
        .swap_name(STATE_DATA);

    let enter_event = format_ident!("{STATE_ENTER_EVENT}");
    let exit_event = format_ident!("{STATE_EXIT_EVENT}");
    let label = format_ident!("{STATE_LABEL}");

    quote!(
        #[allow(non_snake_case)]
        pub mod #name {
            #[warn(non_snake_case)]
            #data_struct
            pub struct #enter_event;
            pub struct #exit_event;
            pub struct #label;
        }
    )
    .into()
}

#[proc_macro]
pub fn game_crate(input: TokenStream) -> TokenStream {
    let input: Empty = syn::parse(input).unwrap();
    let dir = fs::canonicalize(PathBuf::from(try_catch!(
        input,
        env::var("CARGO_MANIFEST_DIR"),
        "No manifest directory specified"
    )))
    .expect("Could not canonicalize manifest directory");

    let data = try_catch!(
        input,
        fs::read_to_string(temp_dir().join(INDEX)),
        "Could not read index file"
    );
    let file = try_catch!(
        input,
        data.split("\n")
            .find_map(|line| {
                line.split_once(INDEX_SEP)
                    .and_then(|(path, file)| (dir == PathBuf::from(path)).then_some(file))
            })
            .ok_or(()),
        format!("Could not find directory in index: {}", dir.display())
    );
    quote!(include!(#file);).into()
}
