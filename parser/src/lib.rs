#![feature(drain_filter)]
#![feature(hash_drain_filter)]
#![feature(array_methods)]
#![feature(iter_intersperse)]
#![feature(lazy_cell)]
#![allow(unused)]

mod codegen;
mod component_set;
mod parse;
mod resolve;
mod system;
mod utils;

use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};
use regex::Regex;
use resolve::Items;
use std::{
    collections::HashMap,
    fs::{self, File},
    hash::Hash,
    io::Read,
    path::PathBuf,
};

use component_set::ComponentSetLabels;
use parse::{AstCrate, ComponentSymbol};
use shared::{
    macros::hash_map,
    msg_result::{MsgTrait, Zip2Msgs},
    parsing::ComponentMacroArgs,
    traits::CollectVecInto,
};
use utils::syn::format_code;

use crate::parse::resolve_path;

// Process:
// 1) Parse AST, get mod/crate structure, use statements, and important syntax items
// 2) Populate with macro symbols
// 3) Parse components, events, globals; insert symbols
// 4) Parse component sets; Validate labels; insert symbols
// 5) Parse systems; Validate arguments; insert symbols
// 6) Codegen
pub fn parse(entry: PathBuf) {
    let (mut crates, span_files) = AstCrate::parse(entry);

    let (items, mut errs) = Items::resolve(&mut crates);

    errs.extend(codegen::codegen(&crates, &items));

    // let engine_cr_idx = crates.get_crate_index(Crate::Engine);
    if !errs.is_empty() {
        let writer = StandardStream::stderr(ColorChoice::Always);
        let config = codespan_reporting::term::Config::default();
        for msg in errs {
            match msg {
                utils::Msg::Diagnostic { msg, file, span } => {
                    let diagnostic = Diagnostic::error()
                        .with_message(msg)
                        .with_labels(vec![Label::primary(file, span)]);
                    term::emit(&mut writer.lock(), &config, &span_files, &diagnostic).unwrap();
                }
                utils::Msg::String(msg) => eprintln!("{}", msg),
            }
        }
        panic!("Build failed");
    }
    // match code {
    //     Ok(code) => Ok(write_codegen(crates, code.map_vec_into(|c| c.to_string()))),
    //     Err(errs) => {
    // let errs = errs.join("\n");
    // let err_msg = "Engine build failed, go to the file below for more information";
    // write_codegen(
    //     crates,
    //     crates
    //         .iter_except([crates.get_crate_index(Crate::Macros)])
    //         .map_vec_into(|cr| match cr.idx {
    //             i if i == engine_cr_idx => {
    //                 format!(
    //                     "compile_error!(\"{err_msg}\");\nconst _: &str = \"\n{errs}\n\";"
    //                 )
    //             }
    //             _ => String::new(),
    //         }),
    // )
    //     }
    // }
}
