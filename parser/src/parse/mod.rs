mod ast_crate;
mod ast_file;
mod ast_mod;
mod attributes;

pub use ast_crate::AstCrate;
pub use ast_mod::{
    AstMod, AstUse, DiscardSymbol, HardcodedSymbol, MatchSymbol, Symbol, SymbolType,
};
pub type ModInfo<'a> = (&'a AstMod, &'a AstCrate, &'a Vec<AstCrate>);
