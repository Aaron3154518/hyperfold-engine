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
pub use find_path::{resolve_path, resolve_path_from_crate, resolve_syn_path, ItemPath};
pub use items::*;
use proc_macro2::Span;
use shared::syn::error::{CriticalResult, Error, MutateResults, Note, StrToError, ToError};
pub use symbol::{
    ComponentSymbol, DiscardSymbol, GlobalSymbol, HardcodedSymbol, MatchSymbol, Symbol, SymbolType,
};

pub type ModInfo<'a> = (&'a AstMod, &'a AstCrate, &'a Vec<AstCrate>);
pub type ModInfoMut<'a> = (&'a mut AstMod, &'a mut AstCrate, &'a Vec<AstCrate>);

#[derive(Debug, Copy, Clone)]
pub struct ItemSpan {
    pub span: Span,
    pub m_idx: usize,
    pub cr_idx: usize,
}

impl ItemSpan {
    pub fn new(cr: &AstCrate, m: &AstMod, span: Span) -> Self {
        Self {
            span,
            m_idx: m.idx,
            cr_idx: cr.idx,
        }
    }
}

impl ToError for ItemSpan {
    fn error(&self, msg: impl Into<String>) -> Error {
        msg.error().with_item_span(self)
    }

    fn warning(&self, msg: impl Into<String>) -> Error {
        msg.warning().with_item_span(self)
    }

    fn note(&self, msg: impl Into<String>) -> Note {
        msg.note().with_item_span(self)
    }
}

pub trait WithItemSpan {
    fn with_item_span(self, span: &ItemSpan) -> Self;
}

impl<T> WithItemSpan for T
where
    T: MutateResults,
{
    fn with_item_span(self, span: &ItemSpan) -> Self {
        self.with_span(&span.span).with_mod(span.cr_idx, span.m_idx)
    }
}
