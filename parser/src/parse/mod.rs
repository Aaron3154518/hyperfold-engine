mod ast_crate;
mod ast_file;
mod ast_mod;
mod attributes;
mod find_path;
mod items;
mod symbol;

pub use ast_crate::AstCrate;
pub use ast_mod::{AstMod, AstModType, NewMod};
pub use attributes::AstAttribute;
pub use items::*;
pub use symbol::{
    ComponentSymbol, DiscardSymbol, GlobalSymbol, HardcodedSymbol, MatchSymbol, Symbol, SymbolType,
};
pub type ModInfo<'a> = (&'a AstMod, &'a AstCrate, &'a Vec<AstCrate>);
pub use find_path::{resolve_path, resolve_path_from_crate, resolve_syn_path, ItemPath};
