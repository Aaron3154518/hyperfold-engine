#![feature(result_option_inspect)]

use bindgen;
use bindgen::callbacks::{DeriveInfo, ParseCallbacks};
use ecs_macros::structs::{ComponentType, ComponentTypes};
use quote::ToTokens;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use syn;

#[derive(Default, Debug)]
struct MyCallbacks;

impl ParseCallbacks for MyCallbacks {
    fn add_derives(&self, d: &DeriveInfo<'_>) -> Vec<String> {
        match d.kind {
            bindgen::callbacks::TypeKind::Enum => {
                vec!["FromPrimitive".to_string()]
            }
            _ => vec![],
        }
    }
}
fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let sdl2_path = "sdl/SDL2-devel-2.26.5-VC/SDL2-2.26.5";
    let sdl2_image_path = "sdl/SDL2_image-devel-2.6.3-VC/SDL2_image-2.6.3";

    // Generate bindings for SDL.h
    let bindings = bindgen::Builder::default()
        .header(format!("{}/include/SDL.h", sdl2_path))
        .generate_comments(false)
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .parse_callbacks(Box::new(MyCallbacks))
        .raw_line("use num_derive::FromPrimitive;")
        .generate()
        .expect("Unable to generate SDL bindings");

    // Write the bindings to a file
    bindings
        .write_to_file(out_dir.join("sdl2_bindings.rs"))
        .expect("Error writing SDL2 bindings to file");

    // Generate bindings for SDL_Image.h
    let bindings = bindgen::Builder::default()
        .header(format!("{}/include/SDL_image.h", sdl2_image_path))
        .clang_arg(format!("-I{}/include", sdl2_path))
        .clang_arg("-Wno-everything")
        .generate_comments(false)
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .parse_callbacks(Box::new(MyCallbacks))
        .raw_line("use num_derive::FromPrimitive;")
        .raw_line("use crate::sdl2::*;")
        .allowlist_type("IMG_.*")
        .allowlist_function("IMG_.*")
        .allowlist_recursively(false)
        .generate()
        .expect("Unable to generate bindings for SDL2_image");

    // Write the bindings to a file
    bindings
        .write_to_file(out_dir.join("sdl2_image_bindings.rs"))
        .expect("Error writing SDL2_Image bindings to file");

    // Link to the SDL2 library
    println!("cargo:rustc-link-search={}/lib/x64", sdl2_path);
    println!("cargo:rustc-link-lib=SDL2");
    println!("cargo:rustc-link-lib=SDL2main");
    println!("cargo:rerun-if-changed={}/includes/SDL.h", sdl2_path);

    // Link to the SDL2_image library
    println!("cargo:rustc-link-search={}/lib/x64", sdl2_image_path);
    println!("cargo:rustc-link-lib=SDL2_image");
    println!(
        "cargo:rerun-if-changed={}/includes/SDL_Image.h",
        sdl2_image_path
    );

    // TODO: Doesn't work if a module is declared manually
    let vecs = entry();

    // Get data
    let (comps, comp_data) = get_data(&vecs, |v| &v.components, |c, _| c.to_data());
    let (globals, glob_data) = get_data(&vecs, |v| &v.globals, |g, _| g.to_data());
    let (events, event_data) = get_data(&vecs, |v| &v.events, |e, _| e.to_data());
    let (systs, sys_data) = get_data(
        &vecs,
        |v| &v.systems,
        |s, v| s.to_data(&v.uses, &comps, &globals, &events),
    );

    // Print
    for v in vecs.iter() {
        eprintln!("File: {}", v.to_file());
        eprintln!("Uses: {}", v.uses_string());
        eprintln!("Components: {}", v.components_string());
        eprintln!("Globals: {}", v.globals_string());
        eprintln!("Systems: {}", v.systems_string());
        eprintln!("Events: {}", v.events_string());
        eprintln!("");
    }
    eprintln!("{}", comp_data);
    eprintln!("{}", glob_data);
    eprintln!("{}", sys_data);
    eprintln!("{}", event_data);
    println!("cargo:rustc-env=COMPONENTS={}", comp_data);
    println!("cargo:rustc-env=GLOBALS={}", glob_data);
    println!("cargo:rustc-env=SYSTEMS={}", sys_data);
    println!("cargo:rustc-env=EVENTS={}", event_data);
}

fn get_data<'a, T, F1, F2>(visitors: &'a Vec<Visitor>, get_vec: F1, to_data: F2) -> (Vec<T>, String)
where
    T: 'a + Clone,
    F1: core::ops::Fn(&'a Visitor) -> &'a Vec<T>,
    F2: core::ops::Fn(&T, &'a Visitor) -> String,
{
    let mut res = Vec::new();
    let mut data = Vec::new();
    for vis in visitors.iter() {
        let v = get_vec(vis);
        res.append(&mut v.to_vec());
        data.append(&mut v.iter().map(|t| to_data(t, vis)).collect::<Vec<_>>());
    }
    (res, data.join(" "))
}

fn join_paths<T>(v: &Vec<T>) -> String
where
    T: HasPath,
{
    v.iter()
        .map(|t| t.get_path_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn concat<T: Clone>(mut v1: Vec<T>, mut v2: Vec<T>) -> Vec<T> {
    v1.append(&mut v2);
    v1
}

fn end<T>(v: &Vec<T>) -> usize {
    if v.is_empty() {
        0
    } else {
        v.len() - 1
    }
}

// Visitor
struct Visitor {
    components: Vec<Component>,
    globals: Vec<Component>,
    modules: Vec<String>,
    systems: Vec<Fn>,
    events: Vec<EventMod>,
    path: Vec<String>,
    uses: Vec<(Vec<String>, String)>,
    build_use: Vec<String>,
}

impl Visitor {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            globals: Vec::new(),
            modules: Vec::new(),
            systems: Vec::new(),
            events: Vec::new(),
            // This is empty for main.rs
            path: Vec::new(),
            // Add use paths from using "crate::"
            uses: vec![(vec!["crate".to_string()], String::new())],
            build_use: Vec::new(),
        }
    }

    pub fn to_file(&self) -> String {
        format!(
            "src/{}.rs",
            if self.path.is_empty() {
                // Add main
                "main".to_string()
            } else {
                self.path.join("/")
            }
        )
    }

    pub fn get_mod_path(&self) -> Vec<String> {
        concat(vec!["crate".to_string()], self.path.to_vec())
    }

    pub fn add_component(&mut self, c: Component, ty: ComponentTypes) {
        match ty {
            ComponentTypes::None => &mut self.components,
            ComponentTypes::Global => &mut self.globals,
        }
        .push(c);
    }

    pub fn uses_string(&self) -> String {
        self.uses
            .iter()
            .map(|u| {
                format!(
                    "{}{}",
                    u.0.join("::"),
                    if u.1.is_empty() {
                        String::new()
                    } else {
                        format!(" as {}", u.1)
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n      ")
    }

    pub fn components_string(&self) -> String {
        join_paths(&self.components)
    }

    pub fn globals_string(&self) -> String {
        join_paths(&self.globals)
    }

    pub fn events_string(&self) -> String {
        join_paths(&self.events)
    }

    pub fn systems_string(&self) -> String {
        self.systems
            .iter()
            .map(|s| {
                format!(
                    "{}({})",
                    s.path.last().expect("Function has no name"),
                    s.args
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn find_attribute<'a>(attrs: &'a Vec<syn::Attribute>, attr: &str) -> Option<&'a syn::Attribute> {
    // TODO: validate path
    attrs
        .iter()
        .find(|a| a.path().segments.last().is_some_and(|s| s.ident == attr))
}

fn parse_attr_args(attr: &syn::Attribute) -> Vec<String> {
    match &attr.meta {
        syn::Meta::List(l) => {
            for t in l.to_token_stream() {
                match t {
                    proc_macro2::TokenTree::Group(g) => {
                        return g
                            .stream()
                            .into_iter()
                            .filter_map(|tt| match tt {
                                proc_macro2::TokenTree::Ident(i) => Some(i.to_string()),
                                _ => None,
                            })
                            .collect();
                    }
                    _ => (),
                }
            }
        }
        _ => (),
    }
    Vec::new()
}

impl syn::visit_mut::VisitMut for Visitor {
    fn visit_item_mod_mut(&mut self, i: &mut syn::ItemMod) {
        self.modules.push(i.ident.to_string());
        syn::visit_mut::visit_item_mod_mut(self, i);
    }

    // Components
    fn visit_item_struct_mut(&mut self, i: &mut syn::ItemStruct) {
        if let Some(a) = find_attribute(&i.attrs, "component") {
            self.add_component(
                Component {
                    path: concat(self.get_mod_path(), vec![i.ident.to_string()]),
                },
                ComponentType::from(parse_attr_args(a)).ty,
            );
        }
        syn::visit_mut::visit_item_struct_mut(self, i);
    }

    fn visit_item_type_mut(&mut self, i: &mut syn::ItemType) {
        if let Some(a) = find_attribute(&i.attrs, "component") {
            self.add_component(
                Component {
                    path: concat(self.get_mod_path(), vec![i.ident.to_string()]),
                },
                ComponentType::from(parse_attr_args(a)).ty,
            );
        }
        syn::visit_mut::visit_item_type_mut(self, i);
    }

    // Functions
    fn visit_item_fn_mut(&mut self, i: &mut syn::ItemFn) {
        if let Some(_) = find_attribute(&i.attrs, "system") {
            // Parse function path and args
            self.systems.push(Fn {
                path: concat(self.get_mod_path(), vec![i.sig.ident.to_string()]),
                args: i
                    .sig
                    .inputs
                    .iter()
                    .filter_map(|arg| match arg {
                        syn::FnArg::Typed(t) => {
                            let mut fn_arg = FnArg::new();
                            fn_arg.parse_arg(&self.get_mod_path(), &t);
                            Some(fn_arg)
                        }
                        syn::FnArg::Receiver(_) => None,
                    })
                    .collect(),
            });
        }
        syn::visit_mut::visit_item_fn_mut(self, i);
    }

    // Enums
    fn visit_item_enum_mut(&mut self, i: &mut syn::ItemEnum) {
        if let Some(_) = find_attribute(&i.attrs, "event") {
            self.events.push(EventMod {
                path: concat(self.get_mod_path(), vec![i.ident.to_string()]),
                events: i.variants.iter().map(|v| v.ident.to_string()).collect(),
            });
        }
        syn::visit_mut::visit_item_enum_mut(self, i);
    }

    // Use Statements
    fn visit_item_use_mut(&mut self, i: &mut syn::ItemUse) {
        self.build_use = Vec::new();
        syn::visit_mut::visit_item_use_mut(self, i);
    }

    fn visit_use_path_mut(&mut self, i: &mut syn::UsePath) {
        if i.ident == "super" {
            if self.build_use.is_empty() {
                self.build_use.push("crate".to_string());
                self.build_use
                    .append(&mut self.path[..end(&self.path)].to_vec());
            } else {
                self.build_use.pop();
            }
        } else {
            self.build_use.push(i.ident.to_string());
        }
        syn::visit_mut::visit_use_path_mut(self, i);
        self.build_use.pop();
    }

    fn visit_use_name_mut(&mut self, i: &mut syn::UseName) {
        // Push
        self.build_use.push(i.ident.to_string());
        self.uses.push((self.build_use.to_vec(), String::new()));
        self.build_use.pop();
        syn::visit_mut::visit_use_name_mut(self, i);
    }

    fn visit_use_rename_mut(&mut self, i: &mut syn::UseRename) {
        self.build_use.push(i.rename.to_string());
        self.uses
            .push((self.build_use.to_vec(), i.ident.to_string()));
        self.build_use.pop();
        syn::visit_mut::visit_use_rename_mut(self, i);
    }

    fn visit_use_glob_mut(&mut self, i: &mut syn::UseGlob) {
        self.build_use.push("*".to_string());
        self.uses.push((self.build_use.to_vec(), String::new()));
        self.build_use.pop();
        syn::visit_mut::visit_use_glob_mut(self, i);
    }
}

// Get all possible paths to `path` give the set of use paths/aliases
fn get_possible_use_paths(
    path: &Vec<String>,
    use_paths: &Vec<(Vec<String>, String)>,
) -> Vec<Vec<String>> {
    use_paths
        .iter()
        .map(|(u_path, alias)| match (u_path.last(), path.first()) {
            (Some(u), Some(t)) => {
                if u == "*" {
                    concat(u_path[..end(&u_path)].to_vec(), path.to_vec())
                } else if t == u {
                    if alias.is_empty() {
                        concat(u_path.to_vec(), path[1..].to_vec())
                    } else {
                        concat(
                            u_path[..end(&u_path)].to_vec(),
                            concat(vec![alias.to_string()], path[1..].to_vec()),
                        )
                    }
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        })
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>()
}

// Entity
const ENTITY_PATH: [&str; 4] = ["crate", "ecs", "entity", "Entity"];

trait HasPath {
    fn get_path_str(&self) -> String;
}

// Component
#[derive(Clone, Debug)]
struct Component {
    path: Vec<String>,
}

impl Component {
    pub fn to_data(&self) -> String {
        format!("{}", self.path.join("::"))
    }
}

impl HasPath for Component {
    fn get_path_str(&self) -> String {
        self.path.join("::")
    }
}

// Event
#[derive(Clone, Debug)]
struct EventMod {
    path: Vec<String>,
    events: Vec<String>,
}

impl EventMod {
    pub fn to_data(&self) -> String {
        format!(
            "{}({})",
            self.path.join("::"),
            self.events
                .iter()
                .map(|s| format!("{}", s))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

impl HasPath for EventMod {
    fn get_path_str(&self) -> String {
        self.path.join("::")
    }
}

// TODO:
#[derive(Clone, Debug)]
enum VecArgKind {
    EntityId,
    Component(usize),
}

// Possible function arg types as engine components
#[derive(Clone, Debug)]
enum FnArgKind {
    Unknown,
    EntityId,
    Component(usize),
    Global(usize),
    Event(usize, usize),
    Vector(Vec<(usize, bool)>),
}

impl std::fmt::Display for FnArgKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unknown => "Unknown",
            Self::EntityId => "EntityId",
            Self::Component(_) => "Component",
            Self::Global(_) => "Global",
            Self::Event(_, _) => "Event",
            Self::Vector(_) => "Vector",
        })
    }
}

// Functions, used to map raw arg types to engine components
#[derive(Clone, Debug)]
struct Fn {
    path: Vec<String>,
    args: Vec<FnArg>,
}

impl Fn {
    pub fn to_data(
        &self,
        use_paths: &Vec<(Vec<String>, String)>,
        components: &Vec<Component>,
        globals: &Vec<Component>,
        events: &Vec<EventMod>,
    ) -> String {
        let mut c_set = HashSet::new();
        let mut g_set = HashSet::new();
        let mut has_event = false;
        let mut has_eid = false;
        let mut has_comps = false;
        let mut has_vec = false;
        let mut errs = Vec::new();
        let data = format!(
            "{}({})",
            self.path.join("::"),
            self.args
                .iter()
                .map(|a| {
                    let err_head =
                        format!("In system {}, arg \"{}\"", self.path.join("::"), a.name);
                    let k = a.map_to_objects(use_paths, components, globals, events);
                    format!(
                        "{}:{}:{}",
                        a.name,
                        if a.mutable { "1" } else { "0" },
                        match k {
                            FnArgKind::Unknown => {
                                errs.push(format!("{}: Type was not recognized", err_head));
                                String::new()
                            }
                            FnArgKind::EntityId => {
                                has_eid = true;
                                format!("eid")
                            }
                            FnArgKind::Component(i) => {
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
                            FnArgKind::Global(i) => {
                                if !g_set.insert(i) {
                                    errs.push(format!(
                                        "{}: Duplicate component type, \"{}\"",
                                        err_head,
                                        a.get_type(),
                                    ));
                                }
                                format!("g{}", i)
                            }
                            FnArgKind::Event(ei, vi) => {
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
                            FnArgKind::Vector(v) => {
                                if v.is_empty() {
                                    errs.push(format!(
                                        "{}: Vector \"{}\" has no components",
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
                                        .map(|a| format!("{}{}", if a.1 { "m" } else { "" }, a.0))
                                        .collect::<Vec<_>>()
                                        .join(":")
                                )
                            }
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join(",")
        );
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

impl std::fmt::Display for Fn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_string().as_str())
    }
}

// Possible function arg types: either a type or a Vec<(ty1, ty2, ...)>
#[derive(Clone, Debug)]
enum FnArgType {
    Path(Vec<String>),
    Vector(Vec<FnArg>),
}

impl std::fmt::Display for FnArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FnArgType::Path(p) => f.write_str(p.join("::").as_str()),
            FnArgType::Vector(v) => f.write_str(
                format!(
                    "Vec<({})>",
                    v.iter()
                        .map(|a| format!("{}", a))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .as_str(),
            ),
        }
    }
}

// Stores a single parsed arg, either a type or a Vec<(ty1, ty2, ...)>
#[derive(Clone, Debug)]
struct FnArg {
    ty: FnArgType,
    name: String,
    mutable: bool,
}

impl FnArg {
    pub fn new() -> Self {
        Self {
            ty: FnArgType::Path(Vec::new()),
            name: String::new(),
            mutable: false,
        }
    }

    pub fn get_type(&self) -> String {
        match &self.ty {
            FnArgType::Path(p) => format!("{}", p.join("::")),
            FnArgType::Vector(v) => format!(
                "Vec<{}>",
                v.iter()
                    .map(|a| a.get_type())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    pub fn map_to_objects(
        &self,
        use_paths: &Vec<(Vec<String>, String)>,
        components: &Vec<Component>,
        globals: &Vec<Component>,
        events: &Vec<EventMod>,
    ) -> FnArgKind {
        match &self.ty {
            FnArgType::Path(p) => {
                let poss_paths = get_possible_use_paths(&p, use_paths);
                // Check is EntityID
                poss_paths
                    .iter()
                    .find(|path| **path == ENTITY_PATH)
                    .map(|_| FnArgKind::EntityId)
                    .or(
                        // Component
                        components
                        .iter()
                        .position(|c| poss_paths.iter().find(|path2| c.path == **path2).is_some())
                        .map(|i| FnArgKind::Component(i))
                    ).or (
                        // Global
                        globals
                        .iter()
                        .position(|c| poss_paths.iter().find(|path2| c.path == **path2).is_some())
                        .map(|i| FnArgKind::Global(i))
                    ).or (
                        // Event
                        events.iter().enumerate().find_map(|(i, e)| {
                            // Find matching enum
                            if let Some(path) = poss_paths.iter().find(|path| e.path == path[..end(path)]) {
                                // Find matching variant
                                if let Some(p) = path
                                    .last()
                                    .and_then(|p| e.events.iter().position(|n| p == n))
                                {
                                    return Some(FnArgKind::Event(i, p));
                                }
                            }
                            None
                        })
                    ).map_or(FnArgKind::Unknown, |k| k)
            },
            FnArgType::Vector(v) => {
                FnArgKind::Vector(
                    v.iter()
                        .map(|a| {
                            let k = a.map_to_objects(use_paths, components, globals, events);
                            match k {
                                FnArgKind::Component(i) => {
                                    (i, a.mutable)
                                },
                                _ => panic!(
                                    "In argument {}: Vec arguments can only include Components, but {} is {}",
                                    self.name,
                                    a.get_type(),
                                    k
                                ),
                            }
                        })
                        .collect()
                )
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
                if let Some(args) = p.path.segments.first().and_then(|s| {
                    if s.ident == "Vec" {
                        Some(&s.arguments)
                    } else {
                        None
                    }
                }) {
                    match args {
                        syn::PathArguments::AngleBracketed(ab) => {
                            if let Some(a) = ab.args.first() {
                                match a {
                                    syn::GenericArgument::Type(t) => match t {
                                        syn::Type::Tuple(tup) => {
                                            let mut v = Vec::new();
                                            for tup_ty in tup.elems.iter() {
                                                let mut arg = FnArg::new();
                                                arg.parse_type(super_path, &tup_ty);
                                                v.push(arg);
                                            }
                                            self.ty = FnArgType::Vector(v);
                                        }
                                        _ => (),
                                    },
                                    _ => (),
                                }
                            }
                        }
                        _ => (),
                    }
                } else {
                    let mut v = Vec::new();
                    for s in p.path.segments.iter() {
                        if s.ident == "super" {
                            if v.is_empty() {
                                v.append(&mut super_path[..end(super_path)].to_vec());
                            } else {
                                v.pop();
                            }
                        } else {
                            v.push(s.ident.to_string());
                        }
                    }
                    self.ty = FnArgType::Path(v);
                }
            }
            syn::Type::Reference(r) => {
                self.mutable = r.mutability.is_some();
                self.parse_type(super_path, &r.elem);
            }
            _ => (),
        }
    }
}

impl std::fmt::Display for FnArg {
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

fn entry() -> Vec<Visitor> {
    find_structs(vec!["main".to_string()])
}

fn find_structs(path: Vec<String>) -> Vec<Visitor> {
    let mut res = Vec::new();

    let dir_name = format!("src/{}", path.join("/"));
    let dir_p = Path::new(&dir_name);
    // If it is a folder, visit mod.rs
    let (file_name, neighbor) = if dir_p.exists() {
        (format!("{}/mod.rs", dir_name), false)
    // Otherwise visit it as a file
    } else {
        (format!("{}.rs", dir_name), true)
    };
    let file_p = Path::new(&file_name);
    if file_p.exists() {
        let mut file = File::open(file_p).expect("Unable to open file");
        let mut src = String::new();
        file.read_to_string(&mut src).expect("Unable to read file");
        match syn::parse_file(&src) {
            Ok(mut ast) => {
                let mut vis = Visitor::new();
                if !path.first().is_some_and(|p| p == "main") {
                    vis.path = path.to_vec();
                    // Add implicit use paths derived from using "super::"
                    vis.uses.append(
                        &mut path
                            .iter()
                            .enumerate()
                            .map(|(i, _)| {
                                (
                                    concat(vec!["crate".to_string()], path[..i + 1].to_vec()),
                                    String::new(),
                                )
                            })
                            .collect(),
                    );
                    vis.uses.push((
                        concat(
                            concat(vec!["crate".to_string()], path.to_vec()),
                            vec!["*".to_string()],
                        ),
                        String::new(),
                    ));
                }
                syn::visit_mut::visit_file_mut(&mut vis, &mut ast);
                let mods = vis.modules.to_vec();
                res.push(vis);
                for mod_name in mods.iter() {
                    // If it is a neighbor, replace the last element of path
                    let mut new_path = if neighbor {
                        path[1..].to_vec()
                    // Otherwise just extend the path
                    } else {
                        path.to_vec()
                    };
                    new_path.push(mod_name.to_string());
                    res.append(&mut find_structs(new_path));
                }
            }
            Err(e) => eprintln!("Failed: {}", e),
        }
    }

    res
}
