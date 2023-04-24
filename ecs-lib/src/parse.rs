use std::{cmp::Ordering, path::PathBuf};

use ecs_macros::structs::ComponentArgs;
use num_traits::FromPrimitive;
use proc_macro2::Span;
use quote::{format_ident, ToTokens};
use regex::Regex;

// Match local instance to list
pub fn find(span: Span, name: String, paths: Vec<Vec<&str>>) -> Option<usize> {
    let mut out = crate::Out::new("out4.txt", true);
    let path = span.unwrap().source_file().path();
    if let Some(dir) = path.parent() {
        let mut no_ext = PathBuf::new();
        no_ext.push(dir);
        no_ext.push(path.file_stem().expect("No file prefix"));
        out.write(format!("{:#?}\n{:#?}\n{:#?}\n", no_ext, name, paths));
        let mut idxs = (0..paths.len()).collect::<Vec<_>>();
        // Repeatedly prune impossible paths
        for (i, p) in no_ext.iter().enumerate() {
            let p = p
                .to_str()
                .expect(format!("Could not convert path segment to string: {:#?}", p).as_str());
            idxs.retain(|j| {
                paths.get(*j).is_some_and(|s| {
                    s.get(i)
                        .is_some_and(|s| (*s == "crate" && p == "src") || *s == p)
                })
            });
        }
        // Find this function in the possible paths
        let n = no_ext.iter().count();
        out.write(format!("Remaining: {:#?}\n{}\n", idxs, n));
        idxs.retain(|j| {
            paths
                .get(*j)
                .is_some_and(|s| s.len() == n + 1 && s.last().is_some_and(|s| *s == name))
        });
        out.write(format!("Remaining: {:#?}\n{:#?}\n", idxs, idxs.first()));
        return idxs.first().cloned();
    }
    None
}

// Component parser
#[derive(Clone, Debug)]
pub struct Component {
    pub var: syn::Ident,
    pub ty: syn::Type,
    pub arg_type: ComponentArgs,
}

impl Component {
    pub fn find<'a>(components: &'a Vec<Component>, path: &str) -> Option<&'a Component> {
        let path_vec = path.split("::").collect::<Vec<_>>();
        components.iter().find(|s| {
            let mut tts = Vec::new();
            for tt in s.ty.to_token_stream() {
                match tt {
                    proc_macro2::TokenTree::Ident(i) => tts.push(i.to_string()),
                    _ => (),
                }
            }
            tts == path_vec
        })
    }

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
}

#[derive(Clone, Debug)]
pub struct ComponentArg {
    pub arg: ServiceArg,
    pub idx: usize,
}

impl ComponentArg {
    pub fn eq_ty(&self, other: &Self) -> bool {
        self.arg.is_mut == other.arg.is_mut && self.idx == other.idx
    }
}

#[derive(Clone, Debug)]
pub struct EventArg {
    pub arg: ServiceArg,
    pub e_idx: usize,
    pub v_idx: usize,
}

impl EventArg {
    pub fn eq_ty(&self, other: &Self) -> bool {
        self.arg.is_mut == other.arg.is_mut
            && self.e_idx == other.e_idx
            && self.v_idx == other.v_idx
    }
}

#[derive(Clone, Debug)]
pub struct Service {
    pub path: Vec<String>,
    pub args: Vec<ComponentArg>,
    pub event: Option<EventArg>,
}

impl Service {
    pub fn new() -> Self {
        Self {
            path: Vec::new(),
            args: Vec::new(),
            event: None,
        }
    }

    pub fn sort_args(&mut self) {
        // Sort by component index
        self.args.sort_by(|a1, a2| a1.idx.cmp(&a2.idx));
    }

    pub fn parse(data: String) -> Vec<Self> {
        let parser = ServiceParser::new();
        data.split(" ").map(|s| parser.parse(s)).collect()
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
        match (&self.event, &other.event) {
            (Some(e1), Some(e2)) => e1.eq_ty(e2),
            (None, None) => true,
            _ => false,
        }
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
            if a1.idx < a2.idx {
                return Ordering::Less;
            } else if a1.idx > a2.idx {
                return Ordering::Greater;
            }
        }
        // Sort by mutability
        for (a1, a2) in self.args.iter().zip(self.args.iter()) {
            if !a1.arg.is_mut && a2.arg.is_mut {
                return Ordering::Less;
            } else if a1.arg.is_mut && !a2.arg.is_mut {
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
                let c = components.get(a.idx).expect("Invalid component index");
                Component {
                    var: c.var.to_owned(),
                    ty: syn::parse_str::<syn::Type>(
                        format!(
                            "&{}{}",
                            if a.arg.is_mut { "mut " } else { "" },
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

    pub fn get_event(&self, events: &Vec<EventMod>) -> Option<syn::Path> {
        self.event.as_ref().map(|e| {
            events
                .get(e.e_idx)
                .and_then(|em| {
                    em.events.get(e.v_idx).map(|e| {
                        let mut p = em.get_path();
                        p.segments.push(syn::PathSegment {
                            ident: format_ident!("{}", e),
                            arguments: syn::PathArguments::None,
                        });
                        p
                    })
                })
                .expect("Could not find service event")
        })
    }
}

struct ServiceParser {
    full_r: Regex,
    path_r: Regex,
    args_r: Regex,
}

impl ServiceParser {
    pub fn new() -> Self {
        let arg_r = r"\w+:\d+:(c\d+|e\d+:\d+)";
        Self {
            full_r: Regex::new(
                format!(
                    r"(?P<path>\w+(::\w+)*)\((?P<args>{}(,{})*)?\)",
                    arg_r, arg_r
                )
                .as_str(),
            )
            .expect("Could not parse regex"),
            path_r: Regex::new(r"\w+").expect("Could not parse regex"),
            args_r: Regex::new(
                r"(?P<var>\w+):(?P<mut>\d+):(c(?P<cidx>\d+)|e(?P<eidx1>\d+):(?P<eidx2>\d+))",
            )
            .expect("Could not parse regex"),
        }
    }

    fn parse(&self, data: &str) -> Service {
        let mut s = Service::new();
        if let Some(c) = self.full_r.captures(data) {
            if let Some(p) = c.name("path") {
                // Parse service path
                s.path = self
                    .path_r
                    .find_iter(p.as_str())
                    .map(|m| m.as_str().to_string())
                    .collect::<Vec<_>>();
            }

            if let Some(a) = c.name("args") {
                // Parse service args
                for c in self.args_r.captures_iter(a.as_str()) {
                    let arg = if let Some((v, m)) = c.name("var").zip(c.name("mut")) {
                        ServiceArg {
                            name: v.as_str().to_string(),
                            is_mut: m.as_str() == "1",
                        }
                    } else {
                        continue;
                    };

                    if let Some(i) = c.name("cidx") {
                        s.args.push(ComponentArg {
                            arg,
                            idx: i
                                .as_str()
                                .parse::<usize>()
                                .expect("Could not parse component index"),
                        });
                    } else if let Some((i1, i2)) = c.name("eidx1").zip(c.name("eidx2")) {
                        s.event = Some(EventArg {
                            arg,
                            e_idx: i1
                                .as_str()
                                .parse::<usize>()
                                .expect("Could not parse event index"),
                            v_idx: i2
                                .as_str()
                                .parse::<usize>()
                                .expect("Could not parse event index"),
                        });
                    }
                }
            }
        }
        s.sort_args();
        s
    }
}

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
