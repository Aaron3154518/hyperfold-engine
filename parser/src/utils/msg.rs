use std::ops::Range;

use proc_macro2::Span;
use shared::msg_result;
use syn::spanned::Spanned;

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
    pub fn for_mod(msg: &str, m: &AstMod, span: &(impl Spanned + ?Sized)) -> Self {
        Self::from_span(msg.to_string(), m.span_file, span.span(), m.span_start)
    }

    pub fn for_file(
        msg: &str,
        file: usize,
        span: &(impl Spanned + ?Sized),
        start: Option<usize>,
    ) -> Self {
        Self::from_span(msg.to_string(), file, span.span(), start)
    }

    pub fn from_span(msg: String, file: usize, span: Span, start: Option<usize>) -> Self {
        let start = start.unwrap_or(0);
        match span.to_range() {
            Ok(r) => Self::Diagnostic {
                msg,
                file,
                span: r.start - start..r.end - start,
            },
            Err(e) => Self::String(format!("{e}\n{msg}")),
        }
    }

    pub fn from_str(msg: &str) -> Self {
        Self::String(msg.to_string())
    }
}

pub type MsgResult<T> = msg_result::MsgResult<T, Msg>;
