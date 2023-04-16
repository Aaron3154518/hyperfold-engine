use bindgen;
use bindgen::callbacks::{DeriveInfo, ParseCallbacks};
use quote::{format_ident, quote};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{env, fs};
use syn::spanned::Spanned;

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
    path: Vec<String>,
}

impl Visitor {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            modules: Vec::new(),
            path: Vec::new(),
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
        f.write_str(format!("File: {}\nComponents: {}", self.to_file(), self.to_data()).as_str())
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
