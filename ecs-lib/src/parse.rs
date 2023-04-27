use std::path::PathBuf;

use ecs_macros::structs::ComponentTypes;
use num_traits::FromPrimitive;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use regex::Regex;

// Match local instance to list
pub fn find(span: Span, name: String, paths: Vec<Vec<String>>) -> Option<usize> {
    let path = span.unwrap().source_file().path();
    if let Some(dir) = path.parent() {
        let mut no_ext = PathBuf::new();
        no_ext.push(dir);
        no_ext.push(path.file_stem().expect("No file prefix"));
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
        idxs.retain(|j| {
            paths
                .get(*j)
                .is_some_and(|s| s.len() == n + 1 && s.last().is_some_and(|s| *s == name))
        });
        return idxs.first().cloned();
    }
    None
}

// Component parser
#[derive(Clone, Debug)]
pub struct Component {
    pub var: syn::Ident,
    pub ty: syn::Type,
    pub arg_type: ComponentTypes,
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
                        if let Some(a) = <ComponentTypes as FromPrimitive>::from_u8(
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

// Systems parser
#[derive(Clone, Debug)]
pub struct EventData {
    pub e_idx: usize,
    pub v_idx: usize,
}

#[derive(Clone, Debug)]
pub enum SystemArgData {
    Component { idx: usize },
    Event(EventData),
}

#[derive(Clone, Debug)]
pub struct SystemArg {
    pub name: String,
    pub is_mut: bool,
    pub data: SystemArgData,
}

#[derive(Clone, Debug)]
pub struct SystemArgTokens {
    args: Vec<TokenStream>,
    c_args: Vec<syn::Ident>,
    g_args: Vec<syn::Ident>,
}

impl SystemArgTokens {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            c_args: Vec::new(),
            g_args: Vec::new(),
        }
    }

    pub fn to_quote(&self, f: &syn::Path, cm: &syn::Ident, em: &syn::Ident) -> TokenStream {
        let args = &self.args;
        let c_args = &self.c_args;
        if c_args.is_empty() {
            quote!((
                |cm: &mut #cm, em: &mut #em| {
                    if let Some(e) = em.get_event() {
                        #f(#(#args),*)
                    }
                }
            ))
        } else {
            quote!((
                |cm: &mut #cm, em: &mut #em| {
                    if let Some(e) = em.get_event() {
                        for key in intersect_keys(&[#(get_keys(&cm.#c_args)),*]).iter() {
                            if let (#(Some(#c_args)),*) = (#(cm.#c_args.get_mut(key)),*) {
                                #f(#(#args),*)
                            }
                        }
                    }
                }
            ))
        }
    }
}

#[derive(Clone, Debug)]
pub struct System {
    pub path: Vec<String>,
    pub args: Vec<SystemArg>,
    pub event: Option<EventData>,
}

impl System {
    pub fn new() -> Self {
        Self {
            path: Vec::new(),
            args: Vec::new(),
            event: None,
        }
    }

    pub fn parse(data: String) -> Vec<Self> {
        let parser = SystemParser::new();
        data.split(" ").map(|s| parser.parse(s)).collect()
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

    pub fn get_args(&self, components: &Vec<Component>) -> SystemArgTokens {
        let mut tokens = SystemArgTokens::new();
        for a in self.args.iter() {
            match a.data {
                SystemArgData::Component { idx } => {
                    let c = components.get(idx).expect("Invalid component index");
                    let var = c.var.to_owned();
                    match c.arg_type {
                        ComponentTypes::None => {
                            tokens.args.push(quote!(#var));
                            tokens.c_args.push(var);
                        }
                        ComponentTypes::Global => {
                            tokens.args.push(quote!(&mut cm.#var));
                            tokens.g_args.push(var);
                        }
                    }
                }
                SystemArgData::Event { .. } => tokens.args.push(quote!(e)),
            }
        }
        tokens
    }
}

struct SystemParser {
    full_r: Regex,
    path_r: Regex,
    args_r: Regex,
}

impl SystemParser {
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

    fn parse(&self, data: &str) -> System {
        let mut s = System::new();
        if let Some(c) = self.full_r.captures(data) {
            if let Some(p) = c.name("path") {
                // Parse system path
                s.path = self
                    .path_r
                    .find_iter(p.as_str())
                    .map(|m| m.as_str().to_string())
                    .collect::<Vec<_>>();
            }

            if let Some(a) = c.name("args") {
                // Parse system args
                for c in self.args_r.captures_iter(a.as_str()) {
                    let (name, is_mut) = c
                        .name("var")
                        .zip(c.name("mut"))
                        .map(|(v, m)| (v.as_str().to_string(), m.as_str() == "1"))
                        .expect("Could not parse variable and mutability");

                    if let Some(i) = c.name("cidx") {
                        s.args.push(SystemArg {
                            name,
                            is_mut,
                            data: SystemArgData::Component {
                                idx: i
                                    .as_str()
                                    .parse::<usize>()
                                    .expect("Could not parse component index"),
                            },
                        });
                    } else if let Some((i1, i2)) = c.name("eidx1").zip(c.name("eidx2")) {
                        let data = EventData {
                            e_idx: i1
                                .as_str()
                                .parse::<usize>()
                                .expect("Could not parse event index"),
                            v_idx: i2
                                .as_str()
                                .parse::<usize>()
                                .expect("Could not parse event index"),
                        };
                        s.event = Some(data.to_owned());
                        s.args.push(SystemArg {
                            name,
                            is_mut,
                            data: SystemArgData::Event(data),
                        });
                    }
                }
            }
        }
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
    _1: syn::Token![,],
    cm: syn::Ident,
    _2: syn::Token![,],
    em: syn::Ident,
}

impl Input {
    pub fn get(self) -> (syn::Ident, syn::Ident, syn::Ident) {
        (self.sm, self.cm, self.em)
    }
}

impl syn::parse::Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            sm: input.parse()?,
            _1: input.parse()?,
            cm: input.parse()?,
            _2: input.parse()?,
            em: input.parse()?,
        })
    }
}
