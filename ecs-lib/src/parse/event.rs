use quote::format_ident;
use regex::Regex;

// Event parser
#[derive(Clone, Debug)]
pub struct EventMod {
    pub path: Vec<String>,
    pub events: Vec<String>,
}

impl EventMod {
    pub fn parse(data: String) -> Vec<Self> {
        let r = Regex::new(r"(?P<path>\w+(::\w+)*)\((?P<events>\w+(,\w+)*)\)")
            .expect("Could not parse regex");
        data.split(" ")
            .filter_map(|s| {
                if let Some(c) = r.captures(s) {
                    if let (Some(p), Some(e)) = (c.name("path"), c.name("events")) {
                        return Some(EventMod {
                            path: p.as_str().split("::").map(|s| s.to_string()).collect(),
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
