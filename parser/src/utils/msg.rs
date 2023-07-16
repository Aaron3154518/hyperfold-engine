use std::ops::Range;

use proc_macro2::Span;
use quote::spanned::Spanned;
use shared::msg_result;

use crate::parse::AstMod;

use super::syn::ToRange;

#[derive(Debug, Clone)]
pub enum Msg {
    Diagnostic {
        msg: String,
        file: usize,
        span: Range<usize>,
    },
    String(String),
}

impl Msg {
    pub fn for_mod(msg: &str, m: &AstMod, span: &impl Spanned) -> Self {
        Self::from_span(msg.to_string(), m.span_file, span.__span())
    }

    pub fn for_file(msg: &str, file: usize, span: &impl Spanned) -> Self {
        Self::from_span(msg.to_string(), file, span.__span())
    }

    pub fn from_span(msg: String, file: usize, span: Span) -> Self {
        match (span.to_range()) {
            Ok(span) => Self::Diagnostic { msg, file, span },
            Err(_) => Self::String(msg),
        }
    }

    pub fn from_str(msg: &str) -> Self {
        Self::String(msg.to_string())
    }
}

pub type MsgResult<T> = msg_result::MsgResult<T, Msg>;
