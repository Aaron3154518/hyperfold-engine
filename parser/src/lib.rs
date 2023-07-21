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
use utils::{syn::format_code, SpanFiles};

use crate::parse::resolve_path;

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

            if let Err(codegen_errs) = codegen::codegen(&crates, &items) {
                errs.extend(codegen_errs);
            }

            (errs, span_files)
        }
        Err(errs) => (errs, SpanFiles::new()),
    };

    if !errs.is_empty() {
        let writer = StandardStream::stderr(ColorChoice::Always);
        let config = codespan_reporting::term::Config::default();
        for msg in errs {
            let diagnostic = match msg {
                utils::Msg::Diagnostic { msg, file, span } => Diagnostic::error()
                    .with_message(msg)
                    .with_labels(vec![Label::primary(file, span)]),
                utils::Msg::String(msg) => Diagnostic::error().with_message(msg),
            };
            term::emit(&mut writer.lock(), &config, &span_files, &diagnostic);
        }
        // panic!("Build failed");
    }
}
