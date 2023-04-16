use bindgen;
use bindgen::callbacks::{DeriveInfo, ParseCallbacks};
use quote::ToTokens;
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
    let data = vecs.iter().map(|v| v.to_data()).collect::<Vec<_>>();
    for v in vecs.iter() {
        eprintln!("\n{}", v);
    }
    println!("cargo:rustc-env=COMPONENTS={}", data.join(" "));
}

// Attempt 2
struct Visitor {
    components: Vec<String>,
    modules: Vec<String>,
    services: Vec<Fn>,
    path: Vec<String>,
    uses: Vec<Vec<String>>,
    build_use: Vec<String>,
}

impl Visitor {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            modules: Vec::new(),
            services: Vec::new(),
            path: Vec::new(),
            uses: Vec::new(),
            build_use: Vec::new(),
        }
    }

    pub fn to_file(&self) -> String {
        format!("src/{}.rs", self.path.join("/"))
    }

    pub fn to_mod(&self) -> String {
        format!("{}", self.path.join("::"))
    }

    pub fn to_data(&self) -> String {
        format!("{}::{{{}}}", self.to_mod(), self.components.join(","))
    }
}

impl std::fmt::Display for Visitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "File: {}\nUses: {}\nComponents: {}\nServices: {}",
                self.to_file(),
                self.uses
                    .iter()
                    .map(|u| u.join("::"))
                    .collect::<Vec<_>>()
                    .join("\n      "),
                self.to_data(),
                self.services
                    .iter()
                    .map(|s| s.to_string())
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
                for s in a.meta.path().segments.iter() {
                    if s.ident == "component" {
                        self.components.push(i.ident.to_string());
                    }
                }
            }
            None => (),
        }
        syn::visit_mut::visit_item_struct_mut(self, i);
    }

    // Functions
    fn visit_item_fn_mut(&mut self, i: &mut syn::ItemFn) {
        i.attrs.iter().find(|a| {
            if let Some(_) = a.meta.path().segments.iter().find(|s| s.ident == "system") {
                self.services.push(Fn {
                    name: i.sig.ident.to_string(),
                    args: i
                        .sig
                        .inputs
                        .iter()
                        .map(|arg| match arg {
                            syn::FnArg::Typed(t) => {
                                let mut fn_arg = FnArg::new();
                                fn_arg.parse_arg(&self.path, &t);
                                fn_arg
                            }
                            _ => FnArg::new(),
                        })
                        .filter(|arg| !arg.ty.is_empty())
                        .collect(),
                });
                return true;
            }
            false
        });
        syn::visit_mut::visit_item_fn_mut(self, i);
    }

    // Use Statements
    fn visit_item_use_mut(&mut self, i: &mut syn::ItemUse) {
        self.build_use = Vec::new();
        syn::visit_mut::visit_item_use_mut(self, i);
    }

    fn visit_use_path_mut(&mut self, i: &mut syn::UsePath) {
        if i.ident == "super" {
            if self.build_use.is_empty() {
                self.build_use
                    .append(&mut self.path[..self.path.len() - 1].to_vec());
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
        self.uses.push(self.build_use.to_vec());
        self.build_use.pop();
        syn::visit_mut::visit_use_name_mut(self, i);
    }

    fn visit_use_rename_mut(&mut self, i: &mut syn::UseRename) {
        self.build_use.push(i.rename.to_string());
        self.uses.push(self.build_use.to_vec());
        self.build_use.pop();
        syn::visit_mut::visit_use_rename_mut(self, i);
    }

    fn visit_use_glob_mut(&mut self, i: &mut syn::UseGlob) {
        self.build_use.push("*".to_string());
        self.uses.push(self.build_use.to_vec());
        self.build_use.pop();
        syn::visit_mut::visit_use_glob_mut(self, i);
    }
}

struct Fn {
    name: String,
    args: Vec<FnArg>,
}

impl Fn {
    pub fn to_string(&self) -> String {
        format!(
            "{}({})",
            self.name,
            self.args
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

impl std::fmt::Display for Fn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_string().as_str())
    }
}

struct FnArg {
    ty: Vec<String>,
    name: String,
    ref_cnt: usize,
}
impl FnArg {
    pub fn new() -> Self {
        Self {
            ty: Vec::new(),
            name: String::new(),
            ref_cnt: 0,
        }
    }

    pub fn to_string(&self) -> String {
        format!(
            "{}:{}{}",
            self.name,
            "&".repeat(self.ref_cnt),
            self.ty.join("::")
        )
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
                            self.ty
                                .append(&mut super_path[..super_path.len() - 1].to_vec());
                        } else {
                            self.ty.pop();
                        }
                    } else {
                        self.ty.push(s.ident.to_string());
                    }
                }
            }
            syn::Type::Reference(r) => {
                self.ref_cnt += 1;
                self.parse_type(super_path, &r.elem);
            }
            _ => (),
        }
    }
}

impl std::fmt::Display for FnArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_string().as_str())
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
                vis.path = path.to_vec();
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
