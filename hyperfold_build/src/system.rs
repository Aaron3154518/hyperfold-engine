use std::collections::HashSet;

use hyperfold_shared::{paths::{ENTITY_PATH, COMPONENTS_PATH, LABEL_PATH}, label::LabelType, macro_args::SystemMacroArgs};

use crate::{util::{AddModPath, concatenate, AddPrefix, parse_path}, component::Trait};

use super::{component::{Component, Global}, event::EventMod, util::{get_possible_use_paths, end}};

// Possible vector engine components
#[derive(Clone, Debug)]
pub enum VecArgKind {
    EntityId,
    Component(usize, bool),
}

// Possible function arg types as engine components
#[derive(Clone, Debug)]
pub enum SystemArgKind {
    Unknown,
    EntityId,
    Component(usize),
    Global(usize),
    Trait(usize),
    Event(usize, usize),
    Container(Vec<VecArgKind>),
    Label(Vec<usize>, LabelType),
}

impl std::fmt::Display for SystemArgKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unknown => "Unknown",
            Self::EntityId => "EntityId",
            Self::Component(..) => "Component",
            Self::Global(..) => "Global",
            Self::Trait(..) => "Trait",
            Self::Event(..) => "Event",
            Self::Container(..) => "Container",
            Self::Label(..) => "Label",
        })
    }
}

// Functions, used to map raw arg types to engine components
#[derive(Clone, Debug)]
pub struct System {
    pub path: Vec<String>,
    pub args: Vec<SystemArg>,
    pub macro_args: SystemMacroArgs
}

impl System {
    pub fn to_data(
        &self,
        use_paths: &Vec<(Vec<String>, String)>,
        components: &Vec<Component>,
        globals: &Vec<Global>,
        traits: &Vec<Trait>,
        events: &Vec<EventMod>,
    ) -> String {
        let (mut c_set, mut g_set, mut t_set) = (HashSet::new(), HashSet::new(), HashSet::new());
        let [mut has_event, mut has_eid, mut has_comps, mut has_vec] = [false; 4];
        let mut errs = Vec::new();
        let args = self
            .args
            .iter()
            .map(|a| {
                let err_head = format!("In system {}, arg \"{}\"", self.path.join("::"), a.name);
                let k = a.map_to_objects(use_paths, components, globals, traits, events);
                if self.macro_args.is_init {
                    match k {
                        SystemArgKind::Global(_) => (),
                        _ => errs.push(
                            format!(
                                "{}: Init system may only take globals but \"{}\" is {}",
                                err_head,
                                a.get_type(),
                                k
                            )
                        )
                    }
                }
                format!(
                    "{}:{}:{}",
                    a.name,
                    if a.mutable { "1" } else { "0" },
                    match k {
                        SystemArgKind::Unknown => {
                            errs.push(format!("{}: Type \"{}\" was not recognized", err_head, a.get_type()));
                            String::new()
                        }
                        SystemArgKind::EntityId => {
                            has_eid = true;
                            if a.mutable {
                                errs.push(format!(
                                    "{}: Entity ID cannot be taken mutably",
                                    err_head
                                ));
                            }
                            format!("id")
                        }
                        SystemArgKind::Component(i) => {
                            has_comps = true;
                            if !c_set.insert(i) {
                                errs.push(format!(
                                    "{}: Duplicate component type, \"{}\"",
                                    err_head,
                                    a.get_type(),
                                ));
                            }
                            format!("c{}", i)
                        }
                        SystemArgKind::Global(i) => {
                            if !g_set.insert(i) {
                                errs.push(format!(
                                    "{}: Duplicate component type, \"{}\"",
                                    err_head,
                                    a.get_type(),
                                ));
                            }
                            format!("g{}", i)
                        }
                        SystemArgKind::Trait(i) => {
                            if !t_set.insert(i) {
                                errs.push(format!(
                                    "{}: Duplicate component type, \"{}\"",
                                    err_head,
                                    a.get_type()
                                ));
                            }
                            format!("g{}", i + globals.len())
                        }
                        SystemArgKind::Event(ei, vi) => {
                            if has_event {
                                errs.push(format!(
                                    "{}: Found event, \"{}\", but an event has already been found",
                                    err_head,
                                    a.get_type()
                                ));
                            }
                            has_event = true;
                            format!("e{}:{}", ei, vi)
                        }
                        SystemArgKind::Container(v) => {
                            if v.is_empty() {
                                errs.push(format!(
                                    "{}: Container \"{}\" has no components",
                                    err_head,
                                    a.get_type()
                                ));
                            }
                            if has_vec {
                                errs.push(format!(
                                    "{}: Found vector, \"{}\", but a vector has already been found",
                                    err_head,
                                    a.get_type()
                                ));
                            }
                            has_vec = true;
                            format!(
                                "v{}",
                                v.iter()
                                    .map(|a| match a {
                                        VecArgKind::EntityId => "id".to_string(),
                                        VecArgKind::Component(i, m) =>
                                            format!("{}{}", if *m { "m" } else { "" }, i),
                                    })
                                    .collect::<Vec<_>>()
                                    .join(":")
                            )
                        }
                        SystemArgKind::Label(v, l) => {
                            if v.is_empty() {
                                errs.push(format!(
                                    "{}: Container \"{}\" has no components",
                                    err_head,
                                    a.get_type()
                                ));
                            }
                            format!(
                                "l{}{}",
                                l.to_data(),
                                v.iter()
                                    .map(|i| format!("{}", i))
                                    .collect::<Vec<_>>()
                                    .join(":")
                            )
                        }
                    }
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let data = format!("{}({}){}", self.path.join("::"), args, if self.macro_args.is_init { "i" } else { "" });
        let err_head = format!("In system {}", self.path.join("::"));
        if has_eid && !has_comps {
            errs.push(format!(
                "{}: Cannot take entity ID without any entity components",
                err_head
            ));
        }
        if has_vec && has_comps {
            errs.push(format!(
                "{}: Cannot wrap components in a vector and take them individually",
                err_head
            ));
        }
        if has_vec && has_eid {
            errs.push(format!(
                "{}: Cannot wrap components in a vector and take entity ids",
                err_head
            ));
        }
        if errs.is_empty() {
            data
        } else {
            for err in errs {
                eprintln!("{}", err);
            }
            panic!()
        }
    }
}

impl AddModPath for System {
    fn add_mod_path(&mut self, path: Vec<String>) {
        self.path = concatenate(path, self.path.to_vec())
    }
}

impl AddPrefix for System {
    fn add_prefix(&mut self, prefix: String) {
        for a in self.args.iter_mut(){
            a.add_prefix(prefix.to_string())
        }
    }
}

impl std::fmt::Display for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_string().as_str())
    }
}

// Possible function arg types: either a type or a Vec<(ty1, ty2, ...)>
#[derive(Clone, Debug)]
pub enum SystemArgType {
    Path(Vec<String>),
    Trait(Vec<String>),
    SContainer(Vec<String>, Box<SystemArg>),
    Container(Vec<String>, Vec<SystemArg>),
}

impl AddPrefix for SystemArgType {
    fn add_prefix(&mut self, prefix: String) {
        match self {
            Self::Path(v) | Self::Trait(v) | Self::Container(v, _) | Self::SContainer(v, _)=> {
                if let Some(s) = v.first_mut() {
                    if s == "crate" {
                        *s = prefix.to_string()
                    }
                }
            }
        }

        match self {
            Self::Container(_, args) => {
                for a in args.iter_mut() {
                    a.add_prefix(prefix.to_string())
                }
            },
            Self::SContainer(_, arg) => arg.add_prefix(prefix.to_string()),
             _ => ()
        }
    }
}

impl std::fmt::Display for SystemArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SystemArgType::Path(p) => f.write_str(p.join("::").as_str()),
            SystemArgType::Trait(p) => f.write_str(format!("impl {}", p.join("::")).as_str()),
            SystemArgType::Container(p, v) => f.write_str(
                format!(
                    "{}<({})>",
                    p.join("::"),
                    v.iter()
                        .map(|a| format!("{}", a))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .as_str(),
            ),
            SystemArgType::SContainer(p, a) => f.write_str(
                format!(
                    "{}<({})>",
                    p.join("::"),
                    a
                ).as_str(),
            )
            ,
        }
    }
}

// Stores a single parsed arg, either a type or a Vec<(ty1, ty2, ...)>
#[derive(Clone, Debug)]
pub struct SystemArg {
    ty: SystemArgType,
    name: String,
    mutable: bool,
}

impl SystemArg {
    pub fn new() -> Self {
        Self {
            ty: SystemArgType::Path(Vec::new()),
            name: String::new(),
            mutable: false,
        }
    }

    pub fn get_type(&self) -> String {
        format!("{}", self.ty)
    }

    pub fn map_to_objects(
        &self,
        use_paths: &Vec<(Vec<String>, String)>,
        components: &Vec<Component>,
        globals: &Vec<Global>,
        traits: &Vec<Trait>,
        events: &Vec<EventMod>,
    ) -> SystemArgKind {
        match &self.ty {
            SystemArgType::Path(p) => {
                let poss_paths = get_possible_use_paths(&p, use_paths);
                poss_paths
                    .iter()
                    .find_map(|path| {
                        // Check is EntityID
                        if *path == ENTITY_PATH {
                            Some(SystemArgKind::EntityId)
                        // Component
                        } else if let Some(i) = components.iter().position(|c| c.path == *path) {
                            Some(SystemArgKind::Component(i))
                        // Global
                        } else if let Some((i, g)) = globals.iter().enumerate().find_map(|(i, g)| if g.path == *path { Some((i, g))} else {None}) {
                            if g.args.is_const && self.mutable {
                                panic!("In argument \"{}\": global type \"{}\" is marked const but is taken mutably", self.name, self.get_type())
                            }
                            Some(SystemArgKind::Global(i))
                        } else if let Some((e_i, v_i)) =
                            events.iter().enumerate().find_map(|(i, e)| {
                                // Find matching enum
                                if e.path == path[..end(path, 0)] {
                                    // Find matching variant
                                    if let Some(p) = path
                                        .last()
                                        .and_then(|p| e.events.iter().position(|n| p == n))
                                    {
                                        return Some((i, p));
                                    }
                                }
                                None
                            })
                        {
                            Some(SystemArgKind::Event(e_i, v_i))
                        } else {
                            None
                        }
                    })
                    .map_or(SystemArgKind::Unknown, |k| k)
            }
            SystemArgType::Trait(p) => {
                let poss_paths = get_possible_use_paths(&p, use_paths);
                poss_paths
                    .iter()
                    .find_map(|path| {
                        // Global
                        if let Some((i, g)) = traits.iter().enumerate().find_map(|(i, tr)| if tr.g_trait.path == *path { Some((i, &tr.global)) } else {None}) {
                            if g.args.is_const && self.mutable {
                                panic!("In argument \"{}\": global type \"{}\" is marked const but is taken mutably", self.name, self.get_type())
                            }
                            Some(SystemArgKind::Trait(i))
                        } else {
                            None
                        }
                    })
                    .map_or(SystemArgKind::Unknown, |k| k)
            }
            SystemArgType::Container(p, v) => {
                let poss_paths = get_possible_use_paths(&p, use_paths);
                // Check against container paths
                poss_paths
                    .iter()
                    .find_map(|path| {
                        if *path == COMPONENTS_PATH {
                            Some(SystemArgKind::Container(
                                v.iter()
                                .map(|a| {
                                    let k = a.map_to_objects(use_paths, components, globals, traits, events);
                                    match k {
                                        SystemArgKind::Component(i) => VecArgKind::Component(i, a.mutable),
                                        SystemArgKind::EntityId => {
                                            if a.mutable {
                                                panic!("In argument \"{}\": Entity ID cannot be taken mutably", self.name);
                                            }
                                            VecArgKind::EntityId
                                        },
                                        _ => panic!(
                                            "In argument \"{}\": Container arguments can only include Components or EntityId, but \"{}\" is {}",
                                            self.name,
                                            a.get_type(),
                                            k
                                        ),
                                    }
                                })
                                .collect()
                            ))
                        } else if let Some(l) = LabelType::from(path) {
                            Some(SystemArgKind::Label(
                                v.iter()
                                .map(|a| {
                                    let k = a.map_to_objects(use_paths, components, globals, traits, events);
                                    match k {
                                        SystemArgKind::Component(i) => i,
                                        _ => panic!(
                                            "In argument {}: Label arguments can only include Components, but {} is {}",
                                            self.name,
                                            a.get_type(),
                                            k
                                        ),
                                    }
                                })
                                .collect(),
                                l
                            ))
                        } else {
                            None
                        }
                    })
                    .expect(format!("Unknown container type: \"{}\"", p.join("::")).as_str())
            }
            SystemArgType::SContainer(p, a) => {
                let poss_paths = get_possible_use_paths(&p, use_paths);
                // Check against container paths
                poss_paths
                 .iter()
                 .find_map(|path| {
                     if *path == LABEL_PATH {
                        let k = a.map_to_objects(use_paths, components, globals, traits, events);                        
                        Some(SystemArgKind::Label(
                            match k {
                                SystemArgKind::Component(i) => vec![i],
                                _ => panic!(
                                    "In argument {}: Label arguments can only include Components, but {} is {}",
                                    self.name,
                                    a.get_type(),
                                    k
                                ),
                            },
                            LabelType::And
                        ))
                     } else {
                         None
                     }
                 })
                 .expect(format!("Unknown container type: \"{}\"", p.join("::")).as_str())
            }
        }
    }

    pub fn parse_arg(&mut self, super_path: &Vec<String>, arg: &syn::PatType) {
        if let syn::Pat::Ident(n) = &*arg.pat {
            self.name = n.ident.to_string();
        }
        self.parse_type(super_path, &arg.ty);
    }

    pub fn parse_type(&mut self, super_path: &Vec<String>, ty: &syn::Type) {
        match ty {
            syn::Type::Path(p) => {
                // Type container with generic tuple
                if let Some(syn::PathArguments::AngleBracketed(ab)) =
                    p.path.segments.last().map(|s| &s.arguments)
                {
                    if let Some(a) = ab.args.first() {
                        self.ty = match a {
                            syn::GenericArgument::Type(t) => match t {
                                syn::Type::Tuple(tup) => {
                                    let mut v = Vec::new();
                                    for tup_ty in tup.elems.iter() {
                                        let mut arg = SystemArg::new();
                                        arg.parse_type(super_path, &tup_ty);
                                        v.push(arg);
                                    }
                                    SystemArgType::Container(
                                        parse_path(super_path, &p.path),
                                        v,
                                    )
                                }
                                syn::Type::Path(_) => {
                                    let mut arg = SystemArg::new();
                                    arg.parse_type(super_path, t);
                                    SystemArgType::SContainer(
                                        parse_path(super_path, &p.path),
                                        Box::new(arg),
                                    )
                                }
                                _ => panic!("Unknown generic argument"),
                            },
                            _ => panic!("Missing generic argument"),
                        }
                    }
                // Normal type
                } else {
                    self.ty = SystemArgType::Path(parse_path(super_path, &p.path));
                }
            }
            syn::Type::Reference(r) => {
                self.mutable = r.mutability.is_some();
                self.parse_type(super_path, &r.elem);
            }
            syn::Type::TraitObject(t) => {
                for tpb in t.bounds.iter() {
                    match tpb {
                        syn::TypeParamBound::Trait(tr) => {
                            self.ty = SystemArgType::Trait(parse_path(super_path, &tr.path));
                            break;
                        },
                        _ => (),
                    }
                }
            },
            _ => (),
            
        }
    }
}

impl AddPrefix for SystemArg {
    fn add_prefix(&mut self, prefix: String) {
        self.ty.add_prefix(prefix)
    }
}

impl std::fmt::Display for SystemArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "{}&{}{}",
                if self.name.is_empty() {
                    String::new()
                } else {
                    format!("{}: ", self.name)
                },
                if self.mutable { "mut " } else { "" },
                self.ty
            )
            .as_str(),
        )
    }
}
