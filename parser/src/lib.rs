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

use std::{fmt::Display, io::Write, path::PathBuf, time::Instant};

use codegen::write_codegen;
use diagnostic::{Diagnostic, DiagnosticLevel, Renderer, ResultsTrait};
use resolve::Items;

use component_set::ComponentSetLabels;
use parse::{AstCrate, ComponentSymbol};
use shared::traits::{Call, CollectVec, CollectVecInto, GetSlice};
use utils::paths::Crate;

struct Timer {
    i: Instant,
    i2: Instant,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            i: Instant::now(),
            i2: Instant::now(),
        }
    }

    pub fn step(&mut self, msg: impl Display) {
        eprintln!("{msg}: {:#?}", self.i2.elapsed());
        self.i2 = Instant::now();
    }

    pub fn finish(&self, msg: impl Display) {
        eprintln!("{msg}: {:#?}", self.i.elapsed());
    }
}

// Process:
// 1) Parse AST, get mod/crate structure, use statements, and important syntax items
// 2) Populate with macro symbols
// 3) Parse components, events, globals; insert symbols
// 4) Parse component sets; Validate labels; insert symbols
// 5) Parse systems; Validate arguments; insert symbols
// 6) Codegen
pub fn parse(entry: PathBuf) {
    let mut t = Timer::new();

    let errs = match AstCrate::parse(entry) {
        Ok(mut crates) => {
            t.step("Parsed Crates (Success)");

            let (items, mut errs) = Items::resolve(&mut crates);
            t.step("Resolved Items");

            let macro_cr_idx = crates.get_crate_index(Crate::Macros);

            let code = match codegen::codegen(&crates, &items).record_errs(&mut errs) {
                Some(code) if errs.is_empty() && !crates.has_errors() => crates
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
            t.step("Finished Codegen");

            write_codegen(code).record_errs(&mut errs);
            t.step("Wrote Codegen");

            let mut i = 0;
            for cr in crates.iter_mut() {
                for m in cr.iter_mods_mut() {
                    let file = m.get_file();
                    let renderer = Renderer::new(&file);
                    for err in m.take_errors() {
                        err.subtract_span(&m.span).render(&renderer).emit().unwrap();
                        i += 1;
                    }
                }
            }
            t.step(format!("Emitted {i} Module Diagnostics"));

            errs
        }
        Err(errs) => {
            t.step("Parsed Crates (Failed)");
            errs
        }
    };

    // TODO: don't hardcode file
    let mut i = 0;
    let renderer = Renderer::new("c:\\Users\\aaore\\repos\\hyperfold-games-library\\src\\main.rs");
    for err in errs {
        err.render(&renderer).emit().unwrap();
        i += 1;
    }
    t.step(format!("Emitted {i} Remaining Diagnostics"));

    t.finish("Build Time");
}
