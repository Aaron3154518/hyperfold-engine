use diagnostic::{CombineResults, ToErr};
use proc_macro2::Span;
use syn::spanned::Spanned;

use crate::{
    syn::{
        error::{err, BuildResult, ToError},
        path_to_vec, Parse,
    },
    traits::CollectVec,
};

// Parsing macro args
pub trait ParseFrom<T>
where
    Self: Sized,
{
    fn parse_from(vals: &T) -> BuildResult<Self>;
}

fn parse<T, U>(input: syn::parse::ParseStream) -> BuildResult<T>
where
    T: ParseFrom<Vec<U>>,
    U: syn::parse::Parse,
{
    let mut args = Vec::new();
    while input.parse::<U>().is_ok_and(|u| {
        args.push(u);
        true
    }) && input.parse::<syn::Token![,]>().is_ok()
    {}
    T::parse_from(&args)
}

// Component args
#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub struct ComponentMacroArgs {
    pub is_dummy: bool,
    pub is_singleton: bool,
}

impl Default for ComponentMacroArgs {
    fn default() -> Self {
        Self {
            is_dummy: false,
            is_singleton: false,
        }
    }
}

impl ParseFrom<Vec<syn::Ident>> for ComponentMacroArgs {
    fn parse_from(vals: &Vec<syn::Ident>) -> BuildResult<Self> {
        let mut c = Self::default();
        vals.map_vec(|i| match i.to_string().as_str() {
            "Dummy" => Ok(c.is_dummy = true),
            "Singleton" => Ok(c.is_singleton = true),
            "Const" => i
                .error("Component cannot be Const\nPerhaps you meant to declare this as 'global'?")
                .err(),
            _ => i
                .error(&format!("Unknown macro argument for component: {i}"))
                .err(),
        })
        .combine_results()?;
        Ok(c)
    }
}

impl Parse for ComponentMacroArgs {
    fn parse(input: syn::parse::ParseStream) -> BuildResult<Self> {
        parse(input)
    }
}

// Global args
#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub struct GlobalMacroArgs {
    pub is_dummy: bool,
    pub is_const: bool,
}

impl Default for GlobalMacroArgs {
    fn default() -> Self {
        Self {
            is_dummy: false,
            is_const: false,
        }
    }
}

impl ParseFrom<Vec<syn::Ident>> for GlobalMacroArgs {
    fn parse_from(vals: &Vec<syn::Ident>) -> BuildResult<Self> {
        let mut g = Self::default();
        vals.map_vec(|i| {
            match i.to_string().as_str() {
            "Dummy" => Ok(g.is_dummy = true),
            "Const" => Ok(g.is_const = true),
            "Singleton" => i.error(
                "Global cannot be a Singleton\nPerhaps you meant to declare this as 'component'?",
            ).err(),
            _ => i.error(&format!("Unknown macro argument for global: {i}")).err(),
        }
        })
        .combine_results()?;
        Ok(g)
    }
}

impl Parse for GlobalMacroArgs {
    fn parse(input: syn::parse::ParseStream) -> BuildResult<Self> {
        parse(input)
    }
}

// System args
#[derive(Debug, Clone)]
pub enum SystemMacroArgs {
    Init(),
    System { states: Vec<(Vec<String>, Span)> },
}

impl Default for SystemMacroArgs {
    fn default() -> Self {
        Self::System { states: Vec::new() }
    }
}

impl ParseFrom<Vec<syn::Path>> for SystemMacroArgs {
    fn parse_from(vals: &Vec<syn::Path>) -> BuildResult<Self> {
        let mut is_init = false;
        let states = vals.filter_map_vec(|p| {
            if p.get_ident().is_some_and(|i| i == "Init") {
                is_init = true;
                None
            } else {
                Some((path_to_vec(p), p.span()))
            }
        });
        match is_init {
            true => match &states[..] {
                [] => Ok(Self::Init()),
                slice => Err(slice.map_vec(|(path, span)| {
                    err(&format!(
                        "Unknown macro argument for init system: {}",
                        path.join("::")
                    ))
                    .span(span)
                })),
            },
            false => Ok(Self::System { states }),
        }
    }
}

impl Parse for SystemMacroArgs {
    fn parse(input: syn::parse::ParseStream) -> BuildResult<Self> {
        parse(input)
    }
}
