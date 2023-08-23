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

use std::{collections::HashSet, fmt::Display, io::Write, path::PathBuf, time::Instant};

use codegen::write_codegen;
use diagnostic::{Diagnostic, DiagnosticLevel, ErrorSpan, ResultsTrait};
use resolve::Items;

use component_set::ComponentSetLabels;
use parse::{AstCrate, ComponentSymbol};
use shared::{
    syn::error::{MutateResults, Renderer, WarningResult},
    traits::{Call, CollectVec, CollectVecInto, ExtendInto, GetSlice},
};
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

// TODO: no hardcode
const MAIN: &str = "c:\\Users\\aaore\\repos\\hyperfold-games-library\\src\\main.rs";

// Process:
// 1) Parse AST, get mod/crate structure, use statements, and important syntax items
// 2) Populate with macro symbols
// 3) Parse components, events, globals; insert symbols
// 4) Parse component sets; Validate labels; insert symbols
// 5) Parse systems; Validate arguments; insert symbols
// 6) Codegen
pub fn parse(entry: PathBuf) {
    let mut t = Timer::new();

    let (renderer, errors) = match AstCrate::parse(entry) {
        Ok(WarningResult {
            value: mut crates,
            mut errors,
        }) => {
            t.step("Parsed Crates (Success)");

            let items = Items::resolve(&mut crates).record_errs(&mut errors);
            t.step("Resolved Items");

            let macro_cr_idx = crates.get_crate_index(Crate::Macros);

            let code = match codegen::codegen(&mut crates, &items).record_errs(&mut errors) {
                Some(code) if errors.is_empty() => crates
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

            write_codegen(code).record_errs(&mut errors);
            t.step("Wrote Codegen");

            let mut renderer = Renderer::new(
                MAIN,
                crates
                    .get_main_mod(crates.get_crate_index(Crate::Main))
                    .map(|m| ErrorSpan::from(&m.span))
                    .unwrap_or_default(),
            );
            for (cr_idx, m_idx) in errors
                .iter()
                .fold(HashSet::new(), |mut hs, e| hs.extend_into(e.get_files()))
            {
                let (file, span) = crates
                    .get_mod(cr_idx, m_idx)
                    .map(|m| (m.get_file(), (&m.span).into()))
                    .unwrap_or((MAIN.to_string(), ErrorSpan::default()));
                renderer.add_file(cr_idx, m_idx, file, span);
            }

            (renderer, errors)
        }
        Err(errs) => {
            t.step("Parsed Crates (Failed)");
            (Renderer::new(MAIN, ErrorSpan::default()), errs)
        }
    };

    let mut i = 0;
    for err in errors {
        err.render(&renderer).emit().unwrap();
        i += 1;
    }
    t.step(format!("Emitted {i} Diagnostics"));

    t.finish("Build Time");
}
