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

    pub fn parse(data: String, ty: ComponentTypes) -> Vec<Self> {
        let ty_char = match ty {
            ComponentTypes::None => "c",
            ComponentTypes::Global => "g",
        };
        data.split(" ")
            .enumerate()
            .map(|(i, s)| Component {
                var: format_ident!("{}{}", ty_char, i),
                ty: syn::parse_str::<syn::Type>(s)
                    .expect(format!("Could not parse Component type: {:#?}", s).as_str()),
            })
            .collect()
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
    EntityId,
    Component { idx: usize },
    Event(EventData),
    Vector(Vec<(usize, bool)>),
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
    // Includes reference and mutability
    c_types: Vec<syn::Type>,
    v_mut: Vec<bool>,
    g_args: Vec<syn::Ident>,
    is_vec: bool,
}

impl SystemArgTokens {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            c_args: Vec::new(),
            c_types: Vec::new(),
            v_mut: Vec::new(),
            g_args: Vec::new(),
            is_vec: false,
        }
    }

    fn get_ref_types(&self) -> Vec<syn::Type> {
        self.c_types
            .iter()
            .zip(self.v_mut.iter())
            .map(|(t, m)| {
                syn::parse_str::<syn::Type>(
                    format!(
                        "&{}{}",
                        if *m { "mut " } else { "" },
                        t.to_token_stream().to_string()
                    )
                    .as_str(),
                )
                .expect("Could not parse type")
            })
            .collect()
    }

    pub fn to_quote(
        &self,
        f: &syn::Path,
        cm: &syn::Ident,
        gm: &syn::Ident,
        em: &syn::Ident,
    ) -> TokenStream {
        let args = &self.args;

        let body = if self.c_args.is_empty() {
            quote!(#f(#(#args),*))
        } else if !self.is_vec {
            let c_args = &self.c_args;
            quote!(
                for eid in intersect_keys(&mut [#(get_keys(&cm.#c_args)),*]).iter() {
                    if let (#(Some(#c_args)),*) = (#(cm.#c_args.get_mut(eid)),*) {
                        #f(#(#args),*)
                    }
                }
            )
        } else {
            let c_types = self.get_ref_types();
            let c_arg1 = self.c_args.first().expect("No first component");
            let iter1 = if *self.v_mut.first().expect("No first mutability") {
                format_ident!("iter_mut")
            } else {
                format_ident!("iter")
            };

            // Tail args and getters
            let (c_args, c_gets) = self.c_args[1..]
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    (
                        a,
                        syn::parse_str::<syn::ExprClosure>(
                            format!("|t| &mut t.{}", i + 1).as_str(),
                        )
                        .expect("Could not parse closure"),
                    )
                })
                .unzip::<_, _, Vec<_>, Vec<_>>();
            // intersect or intersect_mut
            let c_intersect = self.v_mut[1..]
                .iter()
                .map(|b| {
                    if *b {
                        format_ident!("intersect_mut")
                    } else {
                        format_ident!("intersect")
                    }
                })
                .collect::<Vec<_>>();
            // v1, v2, ...
            let c_vars = self
                .c_args
                .iter()
                .enumerate()
                .map(|(i, _a)| format_ident!("v{}", i))
                .collect::<Vec<_>>();
            // (Some(v), None, None, ...)
            let c_tuple_init = syn::parse_str::<syn::ExprClosure>(
                format!(
                    "|(k, v)| (k, (Some(v), {}))",
                    ["None"].repeat(self.c_args.len() - 1).join(",")
                )
                .as_str(),
            )
            .expect("Could not parse tuple");
            quote!(
                let mut v = cm.#c_arg1
                    .#iter1()
                    .map(#c_tuple_init)
                    .collect::<HashMap<_, (#(Option<#c_types>),*)>>();
                #(v = #c_intersect(v, &mut cm.#c_args, #c_gets);)*
                let v = v
                    .into_values()
                    .filter_map(|(#(#c_vars),*)| {
                        if let (#(Some(#c_vars)),*) = (#(#c_vars),*) {
                            Some((#(#c_vars),*))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                #f(#(#args),*);
            )
        };
        quote!((
            |cm: &mut #cm, gm: &mut #gm, em: &mut #em| {
                if let Some(e) = em.get_event() {
                    #body
                }
            }
        ))
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
            match &a.data {
                SystemArgData::EntityId => tokens.args.push(quote!(eid)),
                SystemArgData::Component { idx } => {
                    let c = components.get(*idx).expect("Invalid component index");
                    let var = c.var.to_owned();
                    match c.arg_type {
                        ComponentTypes::None => {
                            tokens.args.push(quote!(#var));
                            tokens.c_args.push(var);
                        }
                        ComponentTypes::Global => {
                            tokens.args.push(quote!(&mut gm.#var));
                            tokens.g_args.push(var);
                        }
                    }
                }
                SystemArgData::Event { .. } => tokens.args.push(quote!(e)),
                SystemArgData::Vector(v) => {
                    tokens.args.push(quote!(v));
                    (tokens.c_types, tokens.c_args) = v
                        .iter()
                        .map(|(i, _m)| {
                            let c = components.get(*i).expect("Invalid component index");
                            (c.ty.to_owned(), c.var.to_owned())
                        })
                        .unzip();
                    tokens.v_mut = v.iter().map(|(_i, m)| *m).collect();
                    tokens.is_vec = true;
                }
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
        let arg_r = r"\w+:\d+:(id|c\d+|e\d+:\d+|vm?\d+(:m?\d+)*)";
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
                r"(?P<var>\w+):(?P<mut>\d+):((?P<eid>id)|c(?P<cidx>\d+)|e(?P<eidx1>\d+):(?P<eidx2>\d+)|v(?P<vidxs>m?\d+(:m?\d+)*))",
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
                    } else if let Some(_) = c.name("eid") {
                        s.args.push(SystemArg {
                            name,
                            is_mut,
                            data: SystemArgData::EntityId,
                        });
                    } else if let Some(idxs) = c.name("vidxs") {
                        s.args.push(SystemArg {
                            name,
                            is_mut,
                            data: SystemArgData::Vector(
                                idxs.as_str()
                                    .split(":")
                                    .map(|mut s| {
                                        let mut is_mut = false;
                                        if s.starts_with("m") {
                                            s = s.split_at(1).1;
                                            is_mut = true;
                                        }
                                        (
                                            s.parse::<usize>().expect(
                                                "Could not parse component index in vector",
                                            ),
                                            is_mut,
                                        )
                                    })
                                    .collect(),
                            ),
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
    gm: syn::Ident,
    _3: syn::Token![,],
    em: syn::Ident,
}

impl Input {
    pub fn get(self) -> (syn::Ident, syn::Ident, syn::Ident, syn::Ident) {
        (self.sm, self.cm, self.gm, self.em)
    }
}

impl syn::parse::Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            sm: input.parse()?,
            _1: input.parse()?,
            cm: input.parse()?,
            _2: input.parse()?,
            gm: input.parse()?,
            _3: input.parse()?,
            em: input.parse()?,
        })
    }
}
