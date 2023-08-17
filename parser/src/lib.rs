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
use diagnostic::{Diagnostic, DiagnosticLevel, Renderer, ResultsTrait};
use resolve::Items;

use component_set::ComponentSetLabels;
use parse::{AstCrate, ComponentSymbol};
use shared::traits::{Call, CollectVec, CollectVecInto, GetSlice};
use utils::paths::Crate;

// Process:
// 1) Parse AST, get mod/crate structure, use statements, and important syntax items
// 2) Populate with macro symbols
// 3) Parse components, events, globals; insert symbols
// 4) Parse component sets; Validate labels; insert symbols
// 5) Parse systems; Validate arguments; insert symbols
// 6) Codegen
pub fn parse(entry: PathBuf) {
    return;
    let errs = match AstCrate::parse(entry) {
        Ok(mut crates) => {
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

            for cr in crates.iter_mut() {
                for m in cr.iter_mods_mut() {
                    let file = m.get_file();
                    let renderer = Renderer::new(&file);
                    for err in m.take_errors() {
                        Diagnostic::from_span(
                            err.msg,
                            file,
                            err.level,
                            Some(renderer.render(err.level, &err.msg, &err.span, Vec::new())),
                            err.span,
                        )
                        .emit()
                        .unwrap();
                    }
                }
            }

            errs
        }
        Err(errs) => errs,
    };

    let file = "c:\\Users\\aaore\\repos\\hyperfold-games-library\\src\\main.rs".to_string();
    let level = DiagnosticLevel::Error;
    let span = Default::default();
    let renderer = Renderer::new(&file);
    for msg in errs {
        Diagnostic::from_span(
            msg.msg,
            file,
            level,
            Some(renderer.render(level, &msg.msg, &span, vec![msg.backtrace()])),
            span,
        )
        .emit()
        .unwrap();
    }
}
