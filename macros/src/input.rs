use parser::util::Catch;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parenthesized,
    parse::Parse,
    token::{Colon, Comma},
};

pub fn parse_destruct_tuple(
    body: &syn::parse::ParseBuffer,
) -> Result<(Vec<syn::Ident>, Vec<TokenStream>, Vec<syn::TypePath>), syn::Error> {
    let (mut _comma, mut _colon): (Comma, Colon);

    let (mut vars, mut muts, mut types) = (Vec::new(), Vec::new(), Vec::new());
    loop {
        let var = match body.parse() {
            Ok(i) => i,
            Err(_) => break,
        };
        _colon = body
            .parse()
            .catch(format!("Missing colon after variable: {var}"));
        muts.push(match body.parse::<syn::token::Mut>() {
            Ok(m) => quote!(#m),
            Err(_) => quote!(),
        });
        types.push(
            body.parse()
                .catch(format!("Missing type for variable: {var}")),
        );
        vars.push(var);
        // Break if not comma for next
        _comma = match body.parse() {
            Ok(c) => c,
            Err(_) => break,
        }
    }
    Ok((vars, muts, types))
}

pub struct Input {
    pub func: syn::Ident,
    pub body: syn::Block,
    pub query: syn::TypePath,
    pub event: syn::TypePath,
    pub comp_vars: Vec<syn::Ident>,
    pub comp_mut: Vec<TokenStream>,
    pub comp_types: Vec<syn::TypePath>,
    pub glob_vars: Vec<syn::Ident>,
    pub glob_mut: Vec<TokenStream>,
    pub glob_types: Vec<syn::TypePath>,
}

impl Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut _comma: Comma;
        let func = input.parse().expect("Missing function name");
        _comma = input.parse().expect("Missing comma after function name");
        let query = input.parse().expect("Missing Query type");
        _comma = input.parse().expect("Missing comma after Query type");
        let event = input.parse().expect("Missing event type");
        _comma = input.parse().expect("Missing comma after event type");

        let body;
        parenthesized!(body in input);
        let (comp_vars, comp_mut, comp_types) =
            parse_destruct_tuple(&body).expect("Could not parse component types");

        _comma = input.parse().expect("Missing comma after components list");

        let body;
        parenthesized!(body in input);
        let (glob_vars, glob_mut, glob_types) =
            parse_destruct_tuple(&body).expect("Could not parse global types");

        _comma = input.parse().expect("Missing comma after globals list");
        let body = input.parse().expect("Missing function body");

        Ok(Self {
            func,
            body,
            query,
            event,
            comp_vars,
            comp_mut,
            comp_types,
            glob_vars,
            glob_mut,
            glob_types,
        })
    }
}

pub struct Input2 {
    pub ident: syn::Ident,
    pub vars: Vec<syn::Ident>,
    pub muts: Vec<TokenStream>,
    pub types: Vec<syn::TypePath>,
}

impl Parse for Input2 {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse().expect("Missing struct ident");
        let _: Comma = input.parse().expect("Missing comma after struct ident");
        let (vars, muts, types) = parse_destruct_tuple(input).expect("Could not parse types");

        Ok(Self {
            ident,
            vars,
            muts,
            types,
        })
    }
}
