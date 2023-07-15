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
    msg_result::{MsgResult, MsgTrait, Zip2Msgs},
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
pub fn parse(entry: PathBuf) -> MsgResult<()> {
    let mut crates = AstCrate::parse(entry);

    let (items, errs) = Items::resolve(&mut crates);

    let result = codegen::codegen(&crates, &items);

    MsgResult::new((), errs).zip(result).map(|_| ())
}
