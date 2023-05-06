use std::{
    env,
    fs::File,
    io::{BufRead, BufReader, Write},
};

use hyperfold_build::env::{BuildData, DATA_FILE};
use hyperfold_shared::label::LabelType;
use quote::ToTokens;

pub const AND: usize = LabelType::And as usize;
pub const OR: usize = LabelType::Or as usize;
pub const NAND: usize = LabelType::Nand as usize;
pub const NOR: usize = LabelType::Nor as usize;

// To type
pub fn type_to_type(ty: &syn::Type, r: bool, m: bool) -> syn::Type {
    string_to_type(ty.to_token_stream().to_string(), r, m)
}

pub fn arr_to_type<const N: usize>(path: [&str; N], r: bool, m: bool) -> syn::Type {
    string_to_type(path.join("::"), r, m)
}

pub fn string_to_type(ty: String, r: bool, m: bool) -> syn::Type {
    syn::parse_str::<syn::Type>(
        format!(
            "{}{}{}",
            if r { "&" } else { "" },
            if m { "mut " } else { "" },
            ty
        )
        .as_str(),
    )
    .expect("Could not parse type")
}

// To path
pub fn arr_to_path<const N: usize>(path: [&str; N]) -> syn::Path {
    string_to_path(path.join("::"))
}

pub fn string_to_path(path: String) -> syn::Path {
    syn::parse_str(&path).expect(format!("Could not parse path: {}", path).as_str())
}

// Read from data file
pub fn read_data(data_ty: BuildData) -> String {
    let mut f = BufReader::new(
        File::open(env::temp_dir().join(DATA_FILE)).expect("Could not open data file for reading"),
    );
    let mut str = String::new();
    for _ in 0..(data_ty as usize + 1) {
        str.clear();
        match f.read_line(&mut str) {
            Ok(0) => panic!("Ran out of lines when trying to parse {:#?}", data_ty),
            Err(e) => panic!("Error while trying to parse {:#?}: {}", data_ty, e),
            _ => (),
        }
    }
    // Trim newlines
    str.trim().to_string()
}

// For writing to files
pub struct Out {
    f: std::fs::File,
}

impl Out {
    pub fn new(f: &str, app: bool) -> Self {
        Self {
            f: std::fs::OpenOptions::new()
                .create(true)
                .append(app)
                .write(true)
                .truncate(!app)
                .open(f)
                .expect(format!("Could not open {}", f).as_str()),
        }
    }

    pub fn write(&mut self, s: String) {
        self.f
            .write(s.as_bytes())
            .expect("Could not write to out.txt");
    }
}
