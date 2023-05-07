#![feature(result_option_inspect)]

use bindgen;
use bindgen::callbacks::{DeriveInfo, ParseCallbacks};
use cargo_metadata::MetadataCommand;
use hyperfold_build::ast_parser::AstParser;
use hyperfold_build::env::*;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

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
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("Missing OUT_DIR environment variable"));

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
    println!(
        "cargo:rustc-link-search=hyperfold-engine/{}/lib/x64",
        sdl2_path
    );
    println!("cargo:rustc-link-lib=SDL2");
    println!("cargo:rustc-link-lib=SDL2main");
    println!(
        "cargo:rerun-if-changed=hyperfold-engine/{}/includes/SDL.h",
        sdl2_path
    );

    // Link to the SDL2_image library
    println!(
        "cargo:rustc-link-search=hyperfold-engine/{}/lib/x64",
        sdl2_image_path
    );
    println!("cargo:rustc-link-lib=SDL2_image");
    println!(
        "cargo:rerun-if-changed=hyperfold-engine/{}/includes/SDL_Image.h",
        sdl2_image_path
    );

    let features = MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("Failed to get metadata")
        .packages[0]
        .features
        .keys()
        .filter_map(|k| {
            match env::var(format!("CARGO_FEATURE_{}", k.replace("-", "_").to_uppercase()).as_str())
            {
                Ok(_) => Some(k.to_owned()),
                Err(_) => None,
            }
        })
        .collect::<Vec<_>>();

    let mut parser = AstParser::new();
    parser.parse("hyperfold_engine", Vec::new(), "src/lib.rs", features);
    parser.parse(
        "crate",
        vec!["hyperfold_engine"],
        "../src/main.rs",
        Vec::new(),
    );
    eprintln!("{}", parser);

    let data = parser.get_data();
    eprintln!("{}", data);

    let path = std::env::temp_dir().join(DATA_FILE);
    eprintln!("Writing data to: {:#?}", path);
    let mut f = File::create(&path).expect("Could not open data file");
    f.write_fmt(format_args!(
        "{}\n{}\n{}\n{}\n{}\n",
        data.components_data,
        data.globals_data,
        data.events_data,
        data.systems_data,
        "hyperfold_engine()"
    ))
    .expect("Could not write to data file");
}
