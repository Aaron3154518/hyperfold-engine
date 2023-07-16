use std::ops::Range;

use proc_macro2::Span;
use shared::{
    msg_result,
    traits::{CollectVec, CollectVecInto, RangeTrait},
};
use syn::spanned::Spanned;

use crate::parse::AstMod;

use super::syn::ToRange;

// Message with span but no file
#[derive(Debug, Clone)]
pub enum ParseMsg {
    Diagnostic { msg: String, span: Range<usize> },
    String(String),
}

impl ParseMsg {
    pub fn from_span(msg: &str, span: Span) -> Self {
        match span.to_range() {
            Ok(span) => Self::Diagnostic {
                msg: msg.to_string(),
                span,
            },
            Err(e) => Self::String(format!("{e}\n{msg}")),
        }
    }

    pub fn from_str(msg: &str) -> Self {
        Self::String(msg.to_string())
    }
}

pub type ParseMsgResult<T> = msg_result::MsgResult<T, ParseMsg>;

// Convert ParseMsg to Msg
pub trait ToMsg<T> {
    fn for_file(self, file: usize, mod_start: Option<usize>) -> T;

    fn for_mod(self, m: &AstMod) -> T;
}

impl ToMsg<Msg> for ParseMsg {
    fn for_file(self, file: usize, mod_start: Option<usize>) -> Msg {
        match self {
            ParseMsg::Diagnostic { msg, span } => Msg::from_range(msg, file, span, mod_start),
            ParseMsg::String(msg) => Msg::String(msg),
        }
    }

    fn for_mod(self, m: &AstMod) -> Msg {
        self.for_file(m.span_file, m.span_start)
    }
}

impl ToMsg<Vec<Msg>> for Vec<ParseMsg> {
    fn for_file(self, file: usize, mod_start: Option<usize>) -> Vec<Msg> {
        self.map_vec_into(|msg| msg.for_file(file, mod_start))
    }

    fn for_mod(self, m: &AstMod) -> Vec<Msg> {
        self.map_vec_into(|msg| msg.for_mod(m))
    }
}

impl<T> ToMsg<MsgResult<T>> for ParseMsgResult<T> {
    fn for_file(self, file: usize, mod_start: Option<usize>) -> MsgResult<T> {
        self.map_err(|e| e.for_file(file, mod_start))
    }

    fn for_mod(self, m: &AstMod) -> MsgResult<T> {
        self.map_err(|e| e.for_mod(m))
    }
}

// Convert syn::Result to ParseMsg
pub trait CatchSpanErr<T> {
    fn catch_err(self, msg: &str) -> ParseMsgResult<T>;

    fn catch_err_span(self, msg: &str, span: Span) -> ParseMsgResult<T>;
}

impl<T> CatchSpanErr<T> for syn::Result<T> {
    fn catch_err(self, msg: &str) -> ParseMsgResult<T> {
        self.map_err(|e| vec![ParseMsg::from_span(msg, e.span())])
    }

    fn catch_err_span(self, msg: &str, span: Span) -> ParseMsgResult<T> {
        self.map_err(|_| vec![ParseMsg::from_span(msg, span)])
    }
}

// Msg with full span information
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
        match span.to_range() {
            Ok(r) => Self::from_range(msg, file, r, start),
            Err(e) => Self::String(format!("{e}\n{msg}")),
        }
    }

    pub fn from_range(msg: String, file: usize, range: Range<usize>, start: Option<usize>) -> Self {
        Self::Diagnostic {
            msg,
            file,
            span: range.sub_into(start.unwrap_or(0)),
        }
    }

    pub fn from_str(msg: &str) -> Self {
        Self::String(msg.to_string())
    }
}

pub type MsgResult<T> = msg_result::MsgResult<T, Msg>;
