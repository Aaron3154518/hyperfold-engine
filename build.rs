use bindgen::callbacks::{DeriveInfo, ParseCallbacks};
use std::{env, panic, path::PathBuf};

use parser::{parse::AstCrate, resolve::ItemsCrate};

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

// TODO: hardcoded
const ENGINE_CRATE: &str = "hyperfold-engine";
const SDL2_PATH: &str = "sdl/SDL2-2.26.5";
const SDL2_IMAGE_PATH: &str = "sdl/SDL2_image-2.6.3";
const SDL2_TTF_PATH: &str = "sdl/SDL2_ttf-2.20.2";

fn build_sdl() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("Missing OUT_DIR environment variable"));

    // Generate bindings for SDL.h
    let bindings = bindgen::Builder::default()
        .header(format!("{SDL2_PATH}/include/SDL.h"))
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

    // Link to the SDL2 library
    println!("cargo:rustc-link-search={ENGINE_CRATE}/{SDL2_PATH}/lib/x64");
    println!("cargo:rustc-link-lib=SDL2");
    println!("cargo:rustc-link-lib=SDL2main");
    println!("cargo:rerun-if-changed={ENGINE_CRATE}/{SDL2_PATH}/includes/SDL.h");
}

fn build_sdl_image() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("Missing OUT_DIR environment variable"));

    // Generate bindings for SDL_Image.h
    let bindings = bindgen::Builder::default()
        .header(format!("{SDL2_IMAGE_PATH}/include/SDL_image.h"))
        .clang_arg(format!("-I{SDL2_PATH}/include"))
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

    // Link to the SDL2_image library
    println!("cargo:rustc-link-search={ENGINE_CRATE}/{SDL2_IMAGE_PATH}/lib/x64");
    println!("cargo:rustc-link-lib=SDL2_image");
    println!("cargo:rerun-if-changed={ENGINE_CRATE}/{SDL2_IMAGE_PATH}/includes/SDL_Image.h");
}

fn build_sdl_ttf() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("Missing OUT_DIR environment variable"));

    // Generate bindings for SDL_Image.h
    let bindings = bindgen::Builder::default()
        .header(format!("{SDL2_TTF_PATH}/include/SDL_ttf.h"))
        .clang_arg(format!("-I{SDL2_PATH}/include"))
        .clang_arg("-Wno-everything")
        .generate_comments(false)
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .parse_callbacks(Box::new(MyCallbacks))
        .raw_line("use num_derive::FromPrimitive;")
        .raw_line("use crate::sdl2::*;")
        .allowlist_type("TTF_.*")
        .allowlist_function("TTF_.*")
        .allowlist_type("_TTF_.*")
        .allowlist_function("_TTF_.*")
        .allowlist_recursively(false)
        .generate()
        .expect("Unable to generate bindings for SDL2_ttf");

    // Write the bindings to a file
    bindings
        .write_to_file(out_dir.join("sdl2_ttf_bindings.rs"))
        .expect("Error writing SDL2_ttf bindings to file");

    // Link to the SDL2_image library
    println!("cargo:rustc-link-search={ENGINE_CRATE}/{SDL2_TTF_PATH}/lib/x64");
    println!("cargo:rustc-link-lib=SDL2_ttf");
    println!("cargo:rerun-if-changed={ENGINE_CRATE}/{SDL2_TTF_PATH}/includes/SDL_ttf.h");
}

pub fn main() {
    build_sdl();
    build_sdl_image();
    build_sdl_ttf();

    // TODO: cfg features
    // let features = MetadataCommand::new()
    //     .no_deps()
    //     .exec()
    //     .expect("Failed to get metadata")
    //     .packages[0]
    //     .features
    //     .keys()
    //     .filter_map(|k| {
    //         match env::var(format!("CARGO_FEATURE_{}", k.replace("-", "_").to_uppercase()).as_str())
    //         {
    //             Ok(_) => Some(k.to_owned()),
    //             Err(_) => None,
    //         }
    //     })
    //     .collect::<Vec<_>>();

    if let Err(e) = panic::catch_unwind(|| {
        // TODO: hardcoded
        let (mut crates, paths) = AstCrate::parse(PathBuf::from("../"));

        // eprintln!("{crates:#?}");

        let mut items = ItemsCrate::parse(&paths, &mut crates);

        // eprintln!("{items:#?}");

        // ast_codegen::codegen(&paths, &mut items);
    }) {
        // TODO: cleanup
        eprintln!("{e:#?}");
    }
}
