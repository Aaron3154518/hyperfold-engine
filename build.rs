use bindgen;
use bindgen::callbacks::{DeriveInfo, ParseCallbacks};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;
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

    visit_mut(String::from_str("src/main").expect("msg"));
}

fn windows_to_linux_path(windows_path: PathBuf) -> PathBuf {
    let mut linux_path = PathBuf::new();
    for component in windows_path.components() {
        match component {
            std::path::Component::RootDir => {
                linux_path.push("/");
            }
            std::path::Component::Normal(os_str) => {
                let s = os_str.to_string_lossy();
                linux_path.push(s.replace("\\", "/"));
            }
            _ => {}
        }
    }
    linux_path
}

fn visit_mut(file: String) {
    let f = format!("{}.rs", file);
    let p = Path::new(f.as_str());
    // eprintln!("Visit: {}", p.display());
    if p.exists() {
        // eprintln!("F {}", p.display());
        let mut file = File::open(p).expect("Unable to open file");
        let mut src = String::new();
        file.read_to_string(&mut src).expect("Unable to read file");
        match syn::parse_file(&src) {
            Ok(mut ast) => syn::visit_mut::visit_file_mut(&mut MyVisitor {}, &mut ast),
            Err(e) => {
                eprintln!("Failed: {}", e);
                return;
            }
        }
    } else {
        let f = format!("{}", file);
        let p = Path::new(f.as_str());
        if p.exists() {
            // eprintln!("D {}", p.display());
            let files = fs::read_dir(p).expect("msg");
            for file in files {
                let path = windows_to_linux_path(file.expect("msg").path());
                let p_str = format!(
                    "{}\\{}",
                    path.parent().unwrap().to_str().unwrap(),
                    path.file_stem().unwrap().to_str().unwrap()
                )
                .replace("\\", "/");
                if p_str.ends_with("mod") {
                    continue;
                }
                // eprintln!("{}, {}", p_str, path.exists());
                visit_mut(p_str);
            }
        }
    }
}

struct MyVisitor {}

impl syn::visit_mut::VisitMut for MyVisitor {
    fn visit_item_mod_mut(&mut self, i: &mut syn::ItemMod) {
        eprintln!("\nVisiting Mod: {}", i.ident);
        visit_mut(format!("src/{}", i.ident.to_string()));
        syn::visit_mut::visit_item_mod_mut(self, i);
        eprintln!("Leaving Mod: {}\n", i.ident);
    }

    fn visit_item_struct_mut(&mut self, i: &mut syn::ItemStruct) {
        eprintln!("Visiting Item Struct: {}", i.ident);
        syn::visit_mut::visit_item_struct_mut(self, i);
    }
}
