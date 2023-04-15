use bindgen;
use bindgen::callbacks::{DeriveInfo, ParseCallbacks};
use quote::{format_ident, quote};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{env, fs};

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

    let mut vis = MyVisitor::new();
    vis.add_module(&"src".to_string());
    vis.visit(&"main".to_string());
    eprintln!("Component Manager:\n{}", vis.manager);
    eprintln!("\nComponents:");
    for comp in vis.components.iter() {
        eprintln!("{}", comp)
    }
    let mut uses = vis
        .components
        .iter()
        .map(|c| {
            c.modules
                .0
                .iter()
                .map(|m| format_ident!("{}", m))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    uses.sort();
    uses.dedup();
    // TODO: uses in types
    let types = vis
        .components
        .iter()
        .map(|c| format_ident!("{}", c.name))
        .collect::<Vec<_>>();
    let vars = types
        .iter()
        .enumerate()
        .map(|(i, _t)| format_ident!("c{}", i))
        .collect::<Vec<_>>();
    let code = quote!(
        #(use #(#uses)::*;)*
        struct ComponentManager {
            #(
                #vars: Vec<#types>
            ),*
        }
    );
    // TODO: Be smarter
    let code = format!("{}", code)
        .replace(" :: ", "::")
        .replace(" ;", ";\n")
        .replace(" < ", "<")
        .replace(" > ", ">")
        .replace("{ ", "{\n")
        .replace(", ", ",\n")
        .replace("}", "\n}")
        .replace("\n ", "\n")
        .replace("\nc", "\n\tc");
    eprintln!("\nCode:\n{}", code);
}

#[derive(Debug)]
struct Modules(Vec<String>);

impl Modules {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn clone(&self) -> Self {
        Self(self.0.clone())
    }

    pub fn join(&self, sep: &str) -> String {
        self.0.join(sep).to_string()
    }

    pub fn join_end(&self, sep: &str) -> String {
        format!(
            "{}{}",
            self.join(sep),
            if self.0.is_empty() { "" } else { sep }
        )
    }

    pub fn add(&mut self, m: &String) {
        self.0.push(m.to_string())
    }

    pub fn prefix_file(&self, f: &String) -> String {
        format!("{}{}.rs", self.join_end("/"), f,)
    }

    pub fn prefix_dir(&self, f: &String) -> String {
        format!("{}{}", self.join_end("/"), f,)
    }

    pub fn prefix_modules(&self, s: &String) -> String {
        if let Some(v) = self.0.get(1..) {
            return format!(
                "{}{}{}",
                v.join("::"),
                if v.is_empty() { "" } else { "::" },
                s
            );
        }
        s.to_string()
    }

    pub fn get_use(&self) -> String {
        if let Some(v) = self.0.get(1..) {
            return format!("{}", v.join("::"),);
        }
        String::new()
    }
}

struct Component {
    name: String,
    modules: Modules,
}

impl Component {
    pub fn to_string(&self) -> String {
        self.modules.prefix_modules(&self.name)
    }
}

impl std::fmt::Display for Component {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_string().as_str())
    }
}

struct MyVisitor {
    components: Vec<Component>,
    manager: Component,
    modules: Modules,
}

impl MyVisitor {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            manager: Component {
                name: String::new(),
                modules: Modules::new(),
            },
            modules: Modules::new(),
        }
    }

    pub fn append(&mut self, mut vis: MyVisitor) {
        self.components.append(&mut vis.components);
        if self.manager.name.is_empty() {
            self.manager = vis.manager;
        }
    }

    pub fn add_module(&mut self, m: &String) {
        for comp in self.components.iter_mut() {
            comp.modules.add(m);
        }
        self.manager.modules.add(m);
        self.modules.add(m);
    }

    fn visit(&mut self, name: &String) {
        let f = self.modules.prefix_file(&name);
        let p = Path::new(f.as_str());
        // eprintln!("Visit: {}", p.display());
        if p.exists() {
            // eprintln!("F {}", p.display());
            let mut file = File::open(p).expect("Unable to open file");
            let mut src = String::new();
            file.read_to_string(&mut src).expect("Unable to read file");
            match syn::parse_file(&src) {
                Ok(mut ast) => {
                    syn::visit_mut::visit_file_mut(self, &mut ast);
                }
                Err(e) => eprintln!("Failed: {}", e),
            }
        } else {
            let d = self.modules.prefix_dir(&name);
            let p = Path::new(d.as_str());
            if p.exists() {
                // eprintln!("D {}", p.display());
                let files = fs::read_dir(p).expect("msg");
                for f in files {
                    let path = f.expect("msg").path();
                    let p_str = format!(
                        "{}\\{}",
                        path.parent().unwrap().to_str().unwrap(),
                        path.file_stem().unwrap().to_str().unwrap()
                    )
                    .replace("\\", "/");
                    if p_str.ends_with("mod") {
                        self.add_module(name);
                        self.visit(&"mod".to_string());
                    }
                }
            }
        }
    }
}

impl syn::visit_mut::VisitMut for MyVisitor {
    fn visit_item_mod_mut(&mut self, i: &mut syn::ItemMod) {
        // eprintln!("Mod: {}{}", self.modules.join_end("::"), i.ident);
        let mut vis = MyVisitor::new();
        vis.modules = self.modules.clone();
        vis.visit(&i.ident.to_string());
        self.append(vis);
        syn::visit_mut::visit_item_mod_mut(self, i);
    }

    fn visit_item_struct_mut(&mut self, i: &mut syn::ItemStruct) {
        // eprintln!("Visiting Item Struct: {}", i.ident);
        match i.attrs.first() {
            Some(a) => {
                for s in a.meta.path().segments.iter() {
                    if s.ident == "component" {
                        self.components.push(Component {
                            name: i.ident.to_string(),
                            modules: self.modules.clone(),
                        });
                    } else if s.ident == "component_manager" {
                        self.manager = Component {
                            name: i.ident.to_string(),
                            modules: self.modules.clone(),
                        };
                    }
                }
            }
            None => (),
        }
        syn::visit_mut::visit_item_struct_mut(self, i);
    }
}
