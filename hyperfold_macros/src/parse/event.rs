use hyperfold_build::env::BuildData;
use quote::format_ident;
use regex::Regex;

use super::util::read_data;

// Event parser
#[derive(Clone, Debug)]
pub struct EventMod {
    pub dep: String,
    pub path: Vec<String>,
    pub events: Vec<String>,
}

impl EventMod {
    pub fn parse(dep: String) -> Vec<Self> {
        let r = Regex::new(r"(?P<path>\w+(::\w+)*)\((?P<events>\w+(,\w+)*)\)")
            .expect("Could not parse regex");
        read_data(BuildData::Events)
            .split(" ")
            .filter_map(|s| {
                if let Some(c) = r.captures(s) {
                    if let (Some(p), Some(e)) = (c.name("path"), c.name("events")) {
                        let mut path = p
                            .as_str()
                            .split("::")
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>();
                        let first = path
                            .first_mut()
                            .expect(format!("Could not parse crate for: {}", p.as_str()).as_str());
                        let e_dep = first.to_string();
                        if *first == dep {
                            *first = "crate".to_string()
                        }
                        return Some(EventMod {
                            dep: e_dep,
                            path,
                            events: e.as_str().split(",").map(|s| s.to_string()).collect(),
                        });
                    }
                }
                None
            })
            .collect()
    }

    pub fn get_path(&self) -> syn::Path {
        syn::Path {
            leading_colon: None,
            segments: self
                .path
                .iter()
                .map(|p| syn::PathSegment {
                    ident: format_ident!("{}", p.as_str()),
                    arguments: syn::PathArguments::None,
                })
                .collect(),
        }
    }
}
