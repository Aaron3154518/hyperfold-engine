mod ast_crate;
mod ast_file;
mod ast_mod;
mod attributes;

pub use ast_crate::AstCrate;
pub use ast_mod::{
    AstEnum, AstFunction, AstItem, AstMod, AstUse, ComponentSymbol, DiscardSymbol, GlobalSymbol,
    HardcodedSymbol, MatchSymbol, Symbol, SymbolType,
};
pub use attributes::AstAttribute;
pub type ModInfo<'a> = (&'a AstMod, &'a AstCrate, &'a Vec<AstCrate>);
