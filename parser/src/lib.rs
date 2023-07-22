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

use std::{io::Write, path::PathBuf};

use codegen::write_codegen;
use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    term,
};
use resolve::Items;

use component_set::ComponentSetLabels;
use parse::{AstCrate, ComponentSymbol};
use shared::{
    msg_result::{MsgTrait, ToMsgs, Zip2Msgs},
    traits::{CollectVec, CollectVecInto, GetSlice},
};
use utils::{paths::Crate, SpanFiles};

use crate::utils::writer::{Writer, WriterTrait};

// Process:
// 1) Parse AST, get mod/crate structure, use statements, and important syntax items
// 2) Populate with macro symbols
// 3) Parse components, events, globals; insert symbols
// 4) Parse component sets; Validate labels; insert symbols
// 5) Parse systems; Validate arguments; insert symbols
// 6) Codegen
pub fn parse(entry: PathBuf) {
    let (errs, span_files) = match AstCrate::parse(entry) {
        Ok((mut crates, span_files)) => {
            let (items, mut errs) = Items::resolve(&mut crates);

            let macro_cr_idx = crates.get_crate_index(Crate::Macros);

            let code = match codegen::codegen(&crates, &items).record_errs(&mut errs) {
                Some(code) if errs.is_empty() => crates
                    .iter_except([macro_cr_idx])
                    .zip(code)
                    .map_vec_into(|(cr, c)| (cr, c.to_string())),
                _ => {
                    let engine_cr_idx = crates.get_crate_index(Crate::Engine);
                    crates
                        .iter_except([macro_cr_idx])
                        .enumerate()
                        .map_vec_into(|(i, cr)| {
                            (
                                cr,
                                if i == engine_cr_idx {
                                    format!(
                                        "compile_error!(\"{}\");",
                                        "Encountered errors when compiling",
                                    )
                                } else {
                                    String::new()
                                },
                            )
                        })
                }
            };

            write_codegen(code).record_errs(&mut errs);

            (errs, span_files)
        }
        Err(errs) => (errs, SpanFiles::new()),
    };

    if !errs.is_empty() {
        let mut writer = Writer::empty();
        writer.write(b"\n");

        let config = codespan_reporting::term::Config::default();
        for msg in errs {
            let diagnostic = match msg {
                utils::Msg::Diagnostic { msg, file, span } => Diagnostic::error()
                    .with_message(msg)
                    .with_labels(vec![Label::primary(file, span)]),
                utils::Msg::String(msg) => Diagnostic::error().with_message(msg),
            };
            term::emit(&mut writer, &config, &span_files, &diagnostic);
        }

        let warning = "cargo:warning=";
        let replace = format!("\n{warning}\r{}\r", " ".repeat(warning.len()));
        println!("{}", writer.to_string().replace("\n", replace.as_str()));
    }
}
