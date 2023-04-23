use bindgen;
use bindgen::callbacks::{DeriveInfo, ParseCallbacks};
use ecs_macros::structs::ComponentArgs;
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
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
    let mut vecs = entry();

    // Get Component data
    let mut comps = Vec::new();
    for i in [ComponentArgs::None, ComponentArgs::Global] {
        for v in vecs.iter() {
            comps.append(&mut v.get_components(i));
        }
    }
    let comp_data = comps
        .iter()
        .map(|c| c.to_data())
        .collect::<Vec<_>>()
        .join(" ");

    // Get Event data
    let mut events = Vec::new();
    for v in vecs.iter() {
        events.append(&mut v.get_events());
    }
    let mut ev_names = HashMap::new();
    events
        .iter_mut()
        .for_each(|e| match e.path.last().cloned() {
            Some(k) => match ev_names.get_mut(&k) {
                Some(c) => {
                    e.cnt = *c;
                    *c += 1;
                }
                None => {
                    ev_names.insert(k, 1 as u8);
                }
            },
            None => (),
        });
    let events_data = events
        .iter()
        .map(|e| format!("{},{}", e.path.join("::"), e.cnt))
        .collect::<Vec<_>>()
        .join(" ");

    // Get Service data
    let mut servs = Vec::new();
    let mut serv_data = Vec::new();
    for v in vecs.iter_mut() {
        v.services.retain_mut(|s| {
            s.map_to_objects(&v.uses, &comps, &events);
            s.func.is_valid()
        });
        servs.append(&mut v.services.to_vec());
        serv_data.append(&mut v.services.iter().map(|f| f.to_data()).collect());
    }
    let serv_data = serv_data.join(" ");

    // Print
    for v in vecs.iter() {
        eprintln!("\n{}", v);
    }
    eprintln!("\n{}", comp_data);
    eprintln!("{}", serv_data);
    eprintln!("{}", events_data);
    println!("cargo:rustc-env=COMPONENTS={}", comp_data);
    println!("cargo:rustc-env=SERVICES={}", serv_data);
    println!("cargo:rustc-env=EVENTS={}", events_data);
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

// Component
#[derive(Clone)]
struct Component {
    path: Vec<String>,
    args: ComponentArgs,
}

impl Component {
    pub fn to_data(&self) -> String {
        format!("{}({})", self.path.join("::"), self.args as u8)
    }
}

// Visitor
struct Visitor {
    components: Vec<Component>,
    modules: Vec<String>,
    services: Vec<Service>,
    events: Vec<Event>,
    path: Vec<String>,
    uses: Vec<(Vec<String>, String)>,
    build_use: Vec<String>,
}

impl Visitor {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            modules: Vec::new(),
            services: Vec::new(),
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

    pub fn get_components(&self, a: ComponentArgs) -> Vec<Component> {
        let mut v = self
            .components
            .iter()
            .filter(|c| c.args == a)
            .map(|c| c.to_owned())
            .collect::<Vec<_>>();
        v.iter_mut()
            .for_each(|c| c.path = concat(self.get_mod_path(), c.path.to_vec()));
        v
    }

    pub fn get_events(&self) -> Vec<Event> {
        let mut v = self.events.clone();
        v.iter_mut()
            .for_each(|e| e.path = concat(self.get_mod_path(), e.path.to_vec()));
        v
    }

    pub fn to_mod(&self) -> String {
        format!("{}", self.get_mod_path().join("::"))
    }

    pub fn to_string(&self) -> String {
        format!(
            "{}::{{{}}}",
            self.to_mod(),
            self.components
                .iter()
                .map(|c| c.path.join("::"))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

impl std::fmt::Display for Visitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "File: {}\nUses: {}\nComponents: {}\nServices: {}\nEvents: {}",
                self.to_file(),
                self.uses
                    .iter()
                    .map(|u| format!(
                        "{}{}",
                        u.0.join("::"),
                        if u.1.is_empty() {
                            String::new()
                        } else {
                            format!(" as {}", u.1)
                        }
                    ))
                    .collect::<Vec<_>>()
                    .join("\n      "),
                self.to_string(),
                self.services
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                self.events
                    .iter()
                    .map(|e| e.path.join("::"))
                    .collect::<Vec<_>>()
                    .join(",")
            )
            .as_str(),
        )
    }
}

impl syn::visit_mut::VisitMut for Visitor {
    fn visit_item_mod_mut(&mut self, i: &mut syn::ItemMod) {
        self.modules.push(i.ident.to_string());
        syn::visit_mut::visit_item_mod_mut(self, i);
    }

    fn visit_item_struct_mut(&mut self, i: &mut syn::ItemStruct) {
        match i.attrs.first() {
            Some(a) => {
                let mut is_comp = false;
                let mut args = ComponentArgs::None;
                for t in a.meta.to_token_stream() {
                    match t {
                        proc_macro2::TokenTree::Group(g) => {
                            for tt in g.stream() {
                                if let proc_macro2::TokenTree::Ident(i) = tt {
                                    match i.to_string().as_str() {
                                        "Global" => args = ComponentArgs::Global,
                                        _ => (),
                                    }
                                }
                            }
                        }
                        proc_macro2::TokenTree::Ident(i) => {
                            if i.to_string() == "component" {
                                is_comp = true;
                            }
                        }
                        _ => (),
                    }
                }
                if is_comp {
                    self.components.push(Component {
                        path: vec![i.ident.to_string()],
                        args,
                    });
                }
            }
            None => (),
        }
        syn::visit_mut::visit_item_struct_mut(self, i);
    }

    // Functions
    fn visit_item_fn_mut(&mut self, i: &mut syn::ItemFn) {
        for a in i.attrs.iter() {
            if let Some(_) = a.path().segments.iter().find(|s| s.ident == "system") {
                // Parse function path and args
                let func = Fn {
                    path: concat(self.path.to_vec(), vec![i.sig.ident.to_string()]),
                    args: i
                        .sig
                        .inputs
                        .iter()
                        .map(|arg| match arg {
                            syn::FnArg::Typed(t) => {
                                let mut fn_arg = FnArg::new();
                                fn_arg.parse_arg(&self.get_mod_path(), &t);
                                fn_arg
                            }
                            _ => FnArg::new(),
                        })
                        .filter(|arg| !arg.ty.is_empty())
                        .collect(),
                };
                match &a.meta {
                    syn::Meta::List(l) => {
                        // Parse events
                        let mut events = Vec::new();
                        l.parse_nested_meta(|l| {
                            events.push(
                                l.path
                                    .segments
                                    .iter()
                                    .map(|s| s.ident.to_string())
                                    .collect::<Vec<_>>(),
                            );
                            Ok(())
                        })
                        .expect("Could not parse nested meta");
                        self.services.push(Service {
                            func,
                            events,
                            event_idxs: Vec::new(),
                        });
                    }
                    syn::Meta::Path(_) => {
                        self.services.push(Service::new(func));
                    }
                    _ => (),
                }
                break;
            }
        }
        syn::visit_mut::visit_item_fn_mut(self, i);
    }

    // Enums
    fn visit_item_enum_mut(&mut self, i: &mut syn::ItemEnum) {
        for a in i.attrs.iter() {
            if let Some(_) = a.meta.path().segments.iter().find(|s| s.ident == "event") {
                self.events.push(Event {
                    path: vec![i.ident.to_string()],
                    variants: i
                        .variants
                        .iter()
                        .map(|v| v.ident.to_string())
                        .collect::<Vec<_>>(),
                    cnt: 0,
                });
                break;
            }
        }
        syn::visit_mut::visit_item_enum_mut(self, i);
    }

    // Use Statements
    fn visit_item_use_mut(&mut self, i: &mut syn::ItemUse) {
        self.build_use = vec!["crate".to_string()];
        syn::visit_mut::visit_item_use_mut(self, i);
    }

    fn visit_use_path_mut(&mut self, i: &mut syn::UsePath) {
        if i.ident == "super" {
            if self.build_use.len() <= 1 {
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

// Event
#[derive(Clone)]
struct Event {
    path: Vec<String>,
    variants: Vec<String>,
    cnt: u8,
}

// Services
#[derive(Clone)]
struct Service {
    func: Fn,
    events: Vec<Vec<String>>,
    event_idxs: Vec<Option<(usize, usize)>>,
}

impl Service {
    pub fn new(func: Fn) -> Self {
        Self {
            func,
            events: Vec::new(),
            event_idxs: Vec::new(),
        }
    }

    pub fn to_string(&self) -> String {
        self.to_data()
    }

    pub fn map_to_objects(
        &mut self,
        use_paths: &Vec<(Vec<String>, String)>,
        comp_paths: &Vec<Component>,
        event_paths: &Vec<Event>,
    ) {
        self.func.map_to_components(use_paths, comp_paths);
        self.event_idxs = self
            .events
            .iter()
            .map(|e| {
                let poss_paths = get_possible_use_paths(e, use_paths);
                event_paths.iter().enumerate().find_map(|(i, e)| {
                    // Find matching enum
                    if let Some(path) = poss_paths.iter().find(|path| e.path == path[..end(path)]) {
                        // Find matching variant
                        if let Some(p) = e
                            .variants
                            .iter()
                            .position(|v| path.last().is_some_and(|p| p == v))
                        {
                            return Some((i, p));
                        }
                    }
                    None
                })
            })
            .collect();
    }

    pub fn to_data(&self) -> String {
        format!(
            "crate::{}<{}>({})",
            self.func.path.join("::"),
            self.event_idxs
                .iter()
                .filter_map(|i| i.map(|(e_i, v_i)| format!("{}::{}", e_i, v_i)))
                .collect::<Vec<_>>()
                .join(","),
            self.func
                .args
                .iter()
                .filter_map(|a| a.ty_idx.map(|i| format!(
                    "{}:{}:{}",
                    a.name,
                    if a.mutable { "1" } else { "0" },
                    i
                )))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

impl std::fmt::Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_string().as_str())
    }
}

// Functions
#[derive(Clone)]
struct Fn {
    path: Vec<String>,
    args: Vec<FnArg>,
}

impl Fn {
    pub fn is_valid(&self) -> bool {
        let mut set = HashSet::new();
        for arg in self.args.iter() {
            if arg.ty_idx.is_none() || !set.insert(arg.ty_idx) {
                return false;
            }
        }
        true
    }

    pub fn map_to_components(
        &mut self,
        use_paths: &Vec<(Vec<String>, String)>,
        comp_paths: &Vec<Component>,
    ) {
        for arg in self.args.iter_mut() {
            arg.map_to_component(use_paths, comp_paths);
        }
    }
}

#[derive(Clone)]
struct FnArg {
    ty: Vec<String>,
    ty_idx: Option<usize>,
    name: String,
    mutable: bool,
}

impl FnArg {
    pub fn new() -> Self {
        Self {
            ty: Vec::new(),
            ty_idx: None,
            name: String::new(),
            mutable: false,
        }
    }

    pub fn map_to_component(
        &mut self,
        use_paths: &Vec<(Vec<String>, String)>,
        comp_paths: &Vec<Component>,
    ) {
        let poss_paths = get_possible_use_paths(&self.ty, use_paths);
        self.ty_idx = comp_paths
            .iter()
            .position(|c| poss_paths.iter().find(|path2| c.path == **path2).is_some());
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
                for s in p.path.segments.iter() {
                    if s.ident == "super" {
                        if self.ty.is_empty() {
                            self.ty.append(&mut super_path[..end(super_path)].to_vec());
                        } else {
                            self.ty.pop();
                        }
                    } else {
                        self.ty.push(s.ident.to_string());
                    }
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
                        &mut path[..end(&path)]
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
