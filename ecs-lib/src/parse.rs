use std::cmp::Ordering;

use ecs_macros::structs::ComponentArgs;
use num_traits::FromPrimitive;
use quote::{format_ident, ToTokens};
use regex::Regex;

// Component parser
#[derive(Clone, Debug)]
pub struct Component {
    pub var: syn::Ident,
    pub ty: syn::Type,
    pub arg_type: ComponentArgs,
}

impl Component {
    pub fn parse(data: String) -> Vec<Self> {
        // Extract component names, types, and args
        let r = Regex::new(r"(?P<name>\w+(::\w+)*)\((?P<args>\d+)\)")
            .expect("Could not construct regex");
        data.split(" ")
            .filter_map(|s| {
                if let Some(c) = r.captures(s) {
                    if let (Some(name), Some(args)) = (c.name("name"), c.name("args")) {
                        if let Some(a) = <ComponentArgs as FromPrimitive>::from_u8(
                            args.as_str()
                                .parse::<u8>()
                                .expect("Could not parse component type code"),
                        ) {
                            return Some((name.as_str().to_string(), a));
                        }
                    }
                }
                None
            })
            .enumerate()
            .map(|(i, (s, t))| Component {
                var: format_ident!("c{}", i),
                ty: syn::parse_str::<syn::Type>(s.as_str())
                    .expect(format!("Could not parse Component type: {:#?}", t).as_str()),
                arg_type: t,
            })
            .collect::<Vec<_>>()
    }
}

// Services parser
#[derive(Clone, Debug)]
pub struct ServiceArg {
    pub name: String,
    pub is_mut: bool,
    pub comp_idx: usize,
}

impl ServiceArg {
    pub fn eq_ty(&self, other: &Self) -> bool {
        self.is_mut == other.is_mut && self.comp_idx == other.comp_idx
    }
}

#[derive(Clone, Debug)]
pub struct Service {
    pub path: Vec<String>,
    pub events: Vec<(usize, String)>,
    pub args: Vec<ServiceArg>,
}

impl Service {
    pub fn new() -> Self {
        Self {
            path: Vec::new(),
            events: Vec::new(),
            args: Vec::new(),
        }
    }

    pub fn sort_args(&mut self) {
        // Sort by component index
        self.args.sort_by(|a1, a2| a1.comp_idx.cmp(&a2.comp_idx))
    }

    pub fn parse(data: String) -> Vec<Self> {
        let parser = ServiceParser::new();
        data.split(" ").map(|s| parser.parse(s)).collect::<Vec<_>>()
    }

    pub fn eq_sig(&self, other: &Self) -> bool {
        if self.args.len() != other.args.len() {
            return false;
        }
        for (a1, a2) in self.args.iter().zip(other.args.iter()) {
            if !a1.eq_ty(a2) {
                return false;
            }
        }
        true
    }

    pub fn cmp(&self, other: &Self) -> Ordering {
        // Sort length first
        if self.args.len() < other.args.len() {
            return Ordering::Less;
        } else if self.args.len() > other.args.len() {
            return Ordering::Equal;
        }
        // Sort by type indices
        for (a1, a2) in self.args.iter().zip(other.args.iter()) {
            if a1.comp_idx < a2.comp_idx {
                return Ordering::Less;
            } else if a1.comp_idx > a2.comp_idx {
                return Ordering::Greater;
            }
        }
        // Sort by mutability
        for (a1, a2) in self.args.iter().zip(self.args.iter()) {
            if !a1.is_mut && a2.is_mut {
                return Ordering::Less;
            } else if a1.is_mut && !a2.is_mut {
                return Ordering::Greater;
            }
        }
        Ordering::Equal
    }

    pub fn get_path(&self) -> syn::Path {
        syn::Path {
            leading_colon: None,
            segments: self
                .path
                .iter()
                .map(|s| syn::PathSegment {
                    ident: format_ident!("{}", s),
                    arguments: syn::PathArguments::None,
                })
                .collect(),
        }
    }

    pub fn get_args(&self, components: &Vec<Component>) -> Vec<Component> {
        self.args
            .iter()
            .map(|a| {
                let c = components.get(a.comp_idx).expect("Invalid component index");
                Component {
                    var: c.var.to_owned(),
                    ty: syn::parse_str::<syn::Type>(
                        format!(
                            "&{}{}",
                            if a.is_mut { "mut " } else { "" },
                            c.ty.to_token_stream().to_string()
                        )
                        .as_str(),
                    )
                    .expect("Could not parse type"),
                    arg_type: c.arg_type,
                }
            })
            .collect::<Vec<_>>()
    }

    pub fn get_events(&self, events: &Vec<Event>) -> Vec<syn::Path> {
        self.events
            .iter()
            .filter_map(|(i, v)| {
                if let Some(e) = events.get(*i) {
                    let mut path = e
                        .path
                        .iter()
                        .map(|p| syn::PathSegment {
                            ident: format_ident!("{}", p),
                            arguments: syn::PathArguments::None,
                        })
                        .collect::<syn::punctuated::Punctuated<_, _>>();
                    path.push(syn::PathSegment {
                        ident: format_ident!("{}", v),
                        arguments: syn::PathArguments::None,
                    });
                    return Some(syn::Path {
                        leading_colon: None,
                        segments: path,
                    });
                }
                None
            })
            .collect()
    }
}

struct ServiceParser {
    full_r: Regex,
    path_r: Regex,
    events_r: Regex,
    args_r: Regex,
}

impl ServiceParser {
    pub fn new() -> Self {
        Self {
            full_r: Regex::new(
                format!(
                    "{}{}{}",
                    r"(?P<path>\w+(::\w+)*)",
                    r"<(?P<events>\w+(::\w+)*(,\w+(::\w+)*)*)?>",
                    r"\((?P<args>\w+:\d+:\d+(,\w+:\d+:\d+)*)?\)"
                )
                .as_str(),
            )
            .expect("Could not parse regex"),
            path_r: Regex::new(r"\w+").expect("Could not parse regex"),
            events_r: Regex::new(r"(?P<idx>\d+)::(?P<var>\w+)").expect("Could not parse regex"),
            args_r: Regex::new(r"(?P<var>\w+):(?P<mut>\d+):(?P<idx>\d+)")
                .expect("Could not parse regex"),
        }
    }

    fn parse(&self, data: &str) -> Service {
        let mut s = match self.full_r.captures(data) {
            Some(c) => Service {
                path: match c.name("path") {
                    Some(p) => self
                        .path_r
                        .find_iter(p.as_str())
                        .map(|m| m.as_str().to_string())
                        .collect::<Vec<_>>(),
                    None => Vec::new(),
                },
                events: match c.name("events") {
                    Some(e) => self
                        .events_r
                        .captures_iter(e.as_str())
                        .filter_map(|c| match (c.name("idx"), c.name("var")) {
                            (Some(i), Some(v)) => Some((
                                i.as_str()
                                    .parse::<usize>()
                                    .expect("Couldn't pares event index"),
                                v.as_str().to_string(),
                            )),
                            _ => None,
                        })
                        .collect::<Vec<_>>(),
                    None => Vec::new(),
                },
                args: match c.name("args") {
                    Some(a) => self
                        .args_r
                        .captures_iter(a.as_str())
                        .filter_map(|c| match (c.name("var"), c.name("mut"), c.name("idx")) {
                            (Some(v), Some(m), Some(i)) => Some(ServiceArg {
                                name: v.as_str().to_string(),
                                is_mut: m.as_str() == "1",
                                comp_idx: i
                                    .as_str()
                                    .parse::<usize>()
                                    .expect("Could not parse component index"),
                            }),
                            _ => None,
                        })
                        .collect::<Vec<_>>(),
                    None => Vec::new(),
                },
            },
            _ => Service::new(),
        };
        s.sort_args();
        s
    }
}

// Event parser
#[derive(Clone, Debug)]
pub struct Event {
    path: Vec<String>,
    cnt: u8,
}

impl Event {
    pub fn parse(data: String) -> Vec<Self> {
        let r = Regex::new(r"(?P<path>\w+(::\w+)*),(?P<cnt>\d+)").expect("Could not parse regex");
        data.split(" ")
            .filter_map(|s| {
                if let Some(c) = r.captures(s) {
                    if let (Some(p), Some(c)) = (c.name("path"), c.name("cnt")) {
                        return Some(Event {
                            path: p.as_str().split("::").map(|s| s.to_string()).collect(),
                            cnt: c
                                .as_str()
                                .parse::<u8>()
                                .expect("Could not parse event count"),
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

    pub fn get_type(&self) -> syn::Ident {
        format_ident!("{}{}", self.path.last().map_or("", |s| s), self.cnt)
    }
}

// Input
#[derive(Debug)]
pub struct Input {
    sm: syn::Ident,
    _comma: syn::Token![,],
    cm: syn::Ident,
}

impl Input {
    pub fn get(self) -> (syn::Ident, syn::Ident) {
        (self.sm, self.cm)
    }
}

impl syn::parse::Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            sm: input.parse()?,
            _comma: input.parse()?,
            cm: input.parse()?,
        })
    }
}