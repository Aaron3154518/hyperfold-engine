use std::collections::HashSet;

use ecs_macros::structs::{LabelType, ENTITY_PATH, NUM_LABEL_TYPES};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use regex::Regex;

const AND: usize = LabelType::And as usize;
const OR: usize = LabelType::Or as usize;
const NAND: usize = LabelType::Nand as usize;
const NOR: usize = LabelType::Nor as usize;

pub fn type_to_ref_type(ty: &syn::Type, m: bool) -> syn::Type {
    string_to_ref_type(ty.to_token_stream().to_string(), m)
}

pub fn path_to_ref_type(path: Vec<String>, m: bool) -> syn::Type {
    string_to_ref_type(path.join("::"), m)
}

pub fn string_to_ref_type(ty: String, m: bool) -> syn::Type {
    syn::parse_str::<syn::Type>(format!("&{}{}", if m { "mut " } else { "" }, ty).as_str())
        .expect("Could not parse type")
}

// Component parser
#[derive(Clone, Debug)]
pub enum ComponentParseType {
    Components,
    Globals,
}

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

    pub fn parse(parse_type: ComponentParseType) -> Vec<Self> {
        let (data_key, ty_char) = match parse_type {
            ComponentParseType::Components => ("COMPONENTS", "c"),
            ComponentParseType::Globals => ("GLOBALS", "g"),
        };
        std::env::var(data_key)
            .expect(data_key)
            .split(" ")
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
pub enum VecArgData {
    EntityId,
    Component(usize, bool),
}

#[derive(Clone, Debug)]
pub enum SystemArgData {
    EntityId,
    Component(usize),
    Global(usize),
    Event(EventData),
    Container(Vec<VecArgData>),
    LabelType,
}

#[derive(Clone, Debug)]
pub struct SystemArg {
    pub name: String,
    pub is_mut: bool,
    pub data: SystemArgData,
}

#[derive(Clone, Debug)]
pub enum VecArgTokens {
    EntityId(syn::Type),
    Component(syn::Type, bool),
}

#[derive(Clone, Debug)]
pub struct SystemArgTokens {
    args: Vec<TokenStream>,
    c_args: Vec<syn::Ident>,
    labels: [Vec<syn::Ident>; NUM_LABEL_TYPES],
    // Includes reference and mutability
    v_types: Vec<VecArgTokens>,
    g_args: Vec<syn::Ident>,
    is_vec: bool,
}

impl SystemArgTokens {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            c_args: Vec::new(),
            labels: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            v_types: Vec::new(),
            g_args: Vec::new(),
            is_vec: false,
        }
    }

    fn quote_labels(&self, body: TokenStream) -> TokenStream {
        let ops = [
            (AND, false, true),
            (OR, false, false),
            (NAND, true, false),
            (NOR, true, true),
        ]
        .iter()
        .filter_map(|(ty, neg, and)| {
            let labels = &self.labels[*ty];
            if labels.is_empty() {
                None
            } else {
                let neg = if *neg { quote!(!) } else { quote!() };
                Some(if *and {
                    quote!(#(#neg cm.#labels.contains_key(eid))&&*)
                } else {
                    quote!(#(#neg cm.#labels.contains_key(eid))||*)
                })
            }
        })
        .collect::<Vec<_>>();

        if ops.is_empty() {
            body
        } else {
            quote!(
                if (#((#ops))&&*) {
                    #body
                }
            )
        }
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
            let if_stmt = self.quote_labels(quote!(#f(#(#args),*)));

            quote!(
                for eid in intersect_keys(&mut [#(get_keys(&cm.#c_args)),*]).iter() {
                    if let (#(Some(#c_args),)*) = (#(cm.#c_args.get_mut(eid),)*) {
                        #if_stmt
                    }
                }
            )
        } else {
            // Container argument types
            let v_types = self
                .v_types
                .iter()
                .map(|a| match a {
                    VecArgTokens::EntityId(ty) => ty,
                    VecArgTokens::Component(ty, _) => ty,
                })
                .collect::<Vec<_>>();
            // Get first argument to initialize the result hashmap
            let arg = self.c_args.first().expect("No first component");
            let nones = ["None"].repeat(self.v_types.len() - 1).join(",");
            let (iter, tuple_init) = match self.v_types.first().expect("No first vector types") {
                VecArgTokens::EntityId(_) => ("iter", format!("|k| (k, (None, {}))", nones)),
                VecArgTokens::Component(_, m) => (
                    if *m { "iter_mut" } else { "iter" },
                    format!("|(k, v)| (k, (Some(v), {}))", nones),
                ),
            };
            let iter = format_ident!("{}", iter);
            let tuple_init = syn::parse_str::<syn::ExprClosure>(tuple_init.as_str())
                .expect("Could not parse tuple init closure");

            // Intersect with tail args
            let intersect_stmts = self.c_args[1..]
                .iter()
                .zip(self.v_types[1..].iter())
                .enumerate()
                .filter_map(|(i, (a, ty))| match ty {
                    VecArgTokens::EntityId(_) => None,
                    VecArgTokens::Component(_, m) => Some(
                        syn::parse_str::<syn::ExprCall>(
                            format!(
                                "intersect{}(v, &mut cm.{}, |t| &mut t.{})",
                                if *m { "_mut" } else { "" },
                                a,
                                i + 1
                            )
                            .as_str(),
                        )
                        .expect("Could not parse intersect call"),
                    ),
                })
                .collect::<Vec<_>>();

            // Contsruct final vector
            // v1, v2, ...
            // c_vars only contains v_i where i is not an eid
            let mut c_vars = Vec::new();
            // all_vars contains all v_i
            // all_args replaces eids with "k"
            let (all_vars, all_args) = self
                .c_args
                .iter()
                .zip(self.v_types.iter())
                .enumerate()
                .map(|(i, (_v, ty))| {
                    let v_i = format_ident!("v{}", i);
                    match ty {
                        VecArgTokens::EntityId(_) => (v_i, format_ident!("eid")),
                        VecArgTokens::Component(_, _) => {
                            c_vars.push(v_i.to_owned());
                            (v_i.to_owned(), v_i)
                        }
                    }
                })
                .unzip::<_, _, Vec<_>, Vec<_>>();

            let if_stmt = self.quote_labels(quote!(return Some((#(#all_args,)*));));

            quote!(
                let mut v = cm.#arg
                    .#iter()
                    .map(#tuple_init)
                    .collect::<HashMap<_, (#(Option<#v_types>,)*)>>();
                #(v = #intersect_stmts;)*
                let v = v
                    .into_iter()
                    .filter_map(|(eid, (#(#all_vars,)*))| {
                        if let (#(Some(#c_vars),)*) = (#(#c_vars,)*) {
                            #if_stmt
                        }
                        None
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
    pub labels: [HashSet<usize>; NUM_LABEL_TYPES],
}

impl System {
    pub fn new() -> Self {
        Self {
            path: Vec::new(),
            args: Vec::new(),
            event: None,
            labels: [
                HashSet::new(),
                HashSet::new(),
                HashSet::new(),
                HashSet::new(),
            ],
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

    pub fn get_args(&self, comps: &Vec<Component>, globals: &Vec<Component>) -> SystemArgTokens {
        let mut tokens = SystemArgTokens::new();
        let mut c_idxs = HashSet::new();
        for a in self.args.iter() {
            match &a.data {
                SystemArgData::EntityId => tokens.args.push(quote!(eid)),
                SystemArgData::Component(i) => {
                    let var = comps
                        .get(*i)
                        .expect("Invalid component index")
                        .var
                        .to_owned();
                    tokens.args.push(quote!(#var));
                    tokens.c_args.push(var);
                    c_idxs.insert(*i);
                }
                SystemArgData::Global(i) => {
                    let var = globals
                        .get(*i)
                        .expect("Invalid component index")
                        .var
                        .to_owned();
                    tokens.args.push(quote!(&mut gm.#var));
                    tokens.g_args.push(var);
                }
                SystemArgData::Event { .. } => tokens.args.push(quote!(e)),
                SystemArgData::Container(v) => {
                    tokens.args.push(quote!(v));
                    (tokens.v_types, tokens.c_args) = v
                        .iter()
                        .map(|a| match a {
                            VecArgData::EntityId => (
                                VecArgTokens::EntityId(string_to_ref_type(
                                    ENTITY_PATH.join("::"),
                                    false,
                                )),
                                format_ident!("eids"),
                            ),
                            VecArgData::Component(i, m) => {
                                c_idxs.insert(*i);
                                let c = comps.get(*i).expect("Invalid component index");
                                (
                                    VecArgTokens::Component(type_to_ref_type(&c.ty, *m), *m),
                                    c.var.to_owned(),
                                )
                            }
                        })
                        .unzip();
                    tokens.is_vec = true;
                }
                SystemArgData::LabelType => tokens.args.push(quote!(std::marker::PhantomData)),
            }
        }

        // Label checks
        // Any components are implicitly a part of AND
        let mut and_labels = self.labels[AND].to_owned();
        and_labels.extend(c_idxs.iter());
        // NOR can't include the label, but AND must include the label
        // After this, there are no components in NOR
        if !and_labels.is_disjoint(&self.labels[NOR]) {
            panic!(
                "{}\n{}",
                "A label is in both AND and NOR. The label condition cannot be satisfied",
                "Note that all components are implicitly AND labels"
            )
        }
        tokens.labels[NOR] = self.labels[NOR]
            .iter()
            .map(|i| {
                comps
                    .get(*i)
                    .expect("Invalid component index for label")
                    .var
                    .to_owned()
            })
            .collect();
        // AND must have it, so OR is automatically satisfied
        if self.labels[OR].is_empty() || !and_labels.is_disjoint(&self.labels[OR]) {
            tokens.labels[OR] = Vec::new()
        // NOR must not have it, so OR is automatically checked
        } else {
            tokens.labels[OR] = self.labels[OR]
                .difference(&self.labels[NOR])
                .map(|i| {
                    comps
                        .get(*i)
                        .expect("Invalid component index for label")
                        .var
                        .to_owned()
                })
                .collect();
            // NOR must have none, but OR must have at least one
            if tokens.labels[OR].is_empty() {
                panic!("All labels in OR are also in NOR. The label condition cannot be satisfied")
            }
        }
        // NOR must not have it, so NAND is automatically satisfied
        if self.labels[NAND].is_empty() || !self.labels[NOR].is_disjoint(&self.labels[NAND]) {
            tokens.labels[NAND] = Vec::new()
        // AND must have it, so NAND is automatically checked
        } else {
            tokens.labels[NAND] = self.labels[NAND]
                .difference(&and_labels)
                .map(|i| {
                    comps
                        .get(*i)
                        .expect("Invalid component index for label")
                        .var
                        .to_owned()
                })
                .collect();
            // AND must have all, but NAND must not have at least one
            if tokens.labels[NAND].is_empty() {
                panic!(
                    "{}\n{}",
                    "All labels in NAND are also in AND. The label condition cannot be satisfied",
                    "Note that all components are implicitly AND labels"
                )
            }
        }
        // Remove all components from AND
        tokens.labels[AND] = and_labels
            .difference(&c_idxs)
            .map(|i| {
                comps
                    .get(*i)
                    .expect("Invalid component index for label")
                    .var
                    .to_owned()
            })
            .collect();

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
        let vec_r = r"(m?\d+|id)";
        let arg_r = format!(
            r"\w+:\d+:(id|c\d+|g\d+|e\d+:\d+|v{}(:{})*|l{}\d+(:\d+)*)",
            vec_r,
            vec_r,
            LabelType::regex()
        );
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
                format!(
                    r"(?P<var>\w+):(?P<mut>\d+):({}|c{}|g{}|e{}|v{}|l{})",
                    r"(?P<eid>id)",
                    r"(?P<cidx>\d+)",
                    r"(?P<gidx>\d+)",
                    r"(?P<eidx1>\d+):(?P<eidx2>\d+)",
                    format!(r"(?P<vidxs>{}(:{})*)", vec_r, vec_r),
                    format!(r"(?P<ltype>{})(?P<lidxs>\d+(:\d+)*)", LabelType::regex()),
                )
                .as_str(),
            )
            .expect("Could not parse regex"),
        }
    }

    fn parse(&self, data: &str) -> System {
        let mut s = System::new();
        let c = self
            .full_r
            .captures(data)
            .expect(format!("Could not parse system: {}", data).as_str());
        // Parse system path
        s.path = self
            .path_r
            .find_iter(
                c.name("path")
                    .expect(format!("Could not parse path from system: {}", data).as_str())
                    .as_str(),
            )
            .map(|m| m.as_str().to_string())
            .collect::<Vec<_>>();

        // Parse system args
        for c in self.args_r.captures_iter(
            c.name("args")
                .expect(format!("Could not parse args from system: {}", data).as_str())
                .as_str(),
        ) {
            let (name, is_mut) = c
                .name("var")
                .zip(c.name("mut"))
                .map(|(v, m)| (v.as_str().to_string(), m.as_str() == "1"))
                .expect("Could not parse variable and mutability");

            if let Some(i) = c.name("cidx") {
                s.args.push(SystemArg {
                    name,
                    is_mut,
                    data: SystemArgData::Component(
                        i.as_str()
                            .parse::<usize>()
                            .expect("Could not parse component index"),
                    ),
                });
            } else if let Some(i) = c.name("gidx") {
                s.args.push(SystemArg {
                    name,
                    is_mut,
                    data: SystemArgData::Global(
                        i.as_str()
                            .parse::<usize>()
                            .expect("Could not parse global index"),
                    ),
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
                    data: SystemArgData::Container(
                        idxs.as_str()
                            .split(":")
                            .map(|mut s| {
                                let mut is_mut = false;
                                if s == "id" {
                                    VecArgData::EntityId
                                } else {
                                    if s.starts_with("m") {
                                        s = s.split_at(1).1;
                                        is_mut = true;
                                    }
                                    VecArgData::Component(
                                        s.parse::<usize>()
                                            .expect("Could not parse container index in"),
                                        is_mut,
                                    )
                                }
                            })
                            .collect(),
                    ),
                });
            } else if let (Some(ty), Some(idxs)) = (c.name("ltype"), c.name("lidxs")) {
                s.args.push(SystemArg {
                    name,
                    is_mut,
                    data: SystemArgData::LabelType,
                });
                s.labels
                    .get_mut(
                        LabelType::from_data(ty.as_str()).expect("Invalid label type") as usize,
                    )
                    .expect("Label type index is out of bounds")
                    .extend(
                        idxs.as_str()
                            .split(":")
                            .map(|s| s.parse::<usize>().expect("Could not parse label index")),
                    )
            } else {
                panic!(
                    "Could not parse system arg: {}",
                    c.get(0).map_or("None", |m| m.as_str())
                )
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
