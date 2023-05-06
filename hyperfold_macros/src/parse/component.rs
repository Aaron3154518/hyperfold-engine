use hyperfold_build::env::BuildData;
use quote::{format_ident, ToTokens};

use super::util::read_data;

// Component parser
#[derive(Clone, Debug)]
pub struct Component {
    pub dep: String,
    pub var: syn::Ident,
    pub ty: syn::Type,
}

impl Component {
    pub fn parse_one(ty: &str, i: usize, dep: &String) -> Self {
        let mut path = ty.split("::").collect::<Vec<_>>();
        let first = path
            .first_mut()
            .expect(format!("Could not parse crate for: {}", ty).as_str());
        let c_dep = first.to_string();
        if first == dep {
            *first = "crate"
        }
        Self {
            dep: c_dep,
            var: format_ident!("c{}", i),
            ty: syn::parse_str::<syn::Type>(&path.join("::"))
                .expect(format!("Could not parse Component type: {}", ty).as_str()),
        }
    }

    pub fn parse(dep: String) -> Vec<Self> {
        read_data(BuildData::Components)
            .split(" ")
            .enumerate()
            .map(|(i, s)| Self::parse_one(s, i, &dep))
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct Global {
    pub var: syn::Ident,
    pub ty: syn::Type,
}

impl Global {
    pub fn find<'a>(globals: &'a Vec<Global>, path: &[&str]) -> Option<&'a Global> {
        globals.iter().find(|g| {
            let mut tts = Vec::new();
            for tt in g.ty.to_token_stream() {
                match tt {
                    proc_macro2::TokenTree::Ident(i) => tts.push(i.to_string()),
                    _ => (),
                }
            }
            tts == path
        })
    }

    pub fn parse_one(ty: &str, i: usize) -> Self {
        Self {
            var: format_ident!("g{}", i),
            ty: syn::parse_str::<syn::Type>(ty)
                .expect(format!("Could not parse Component type: {}", ty).as_str()),
        }
    }

    pub fn parse() -> Vec<Self> {
        read_data(BuildData::Globals)
            .split(" ")
            .enumerate()
            .map(|(i, s)| Self::parse_one(s, i))
            .collect()
    }
}
