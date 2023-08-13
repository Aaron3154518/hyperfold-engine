use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::ops::Range;
use syn::spanned::Spanned;

use super::ToRange;
use crate::{
    msg_result,
    traits::{CollectVecInto, RangeTrait},
};

pub trait SpanFile {
    fn span_file(&self) -> usize;

    fn span_start(&self) -> Option<usize>;
}

// Convert span to error
pub trait NewError {
    fn msg(&self, msg: &str) -> ParseMsg;

    fn error<T>(&self, msg: &str) -> DiagnosticResult<T>;
}

impl<S> NewError for S
where
    S: Spanned,
{
    fn msg(&self, msg: &str) -> ParseMsg {
        ParseMsg::from_span(msg, self.span())
    }

    fn error<T>(&self, msg: &str) -> DiagnosticResult<T> {
        Err(vec![self.msg(msg)])
    }
}

// Message with span but no file
#[derive(Debug, Clone)]
pub enum ParseMsg {
    Diagnostic { msg: String, span: Span },
    String(String),
}

impl ParseMsg {
    pub fn from_span(msg: &str, span: Span) -> Self {
        Self::Diagnostic {
            msg: msg.to_string(),
            span,
        }
    }

    pub fn from_str(msg: &str) -> Self {
        Self::String(msg.to_string())
    }
}

pub type DiagnosticResult<T> = msg_result::DiagnosticResult<T, ParseMsg>;

// Convert ParseMsg to Msg
pub trait ToMsg<T> {
    fn for_file(self, file: usize, mod_start: Option<usize>) -> T;

    fn in_mod(self, m: &impl SpanFile) -> T;
}

impl ToMsg<Msg> for ParseMsg {
    fn for_file(self, file: usize, mod_start: Option<usize>) -> Msg {
        match self {
            ParseMsg::Diagnostic { msg, span } => match span.to_range() {
                Ok(range) => Msg::from_range(msg, file, range, mod_start),
                Err(_) => Error::new(&msg),
            },
            ParseMsg::String(msg) => Error::new(&msg),
        }
    }

    fn in_mod(self, m: &impl SpanFile) -> Msg {
        self.for_file(m.span_file(), m.span_start())
    }
}

impl ToMsg<Vec<Msg>> for Vec<ParseMsg> {
    fn for_file(self, file: usize, mod_start: Option<usize>) -> Vec<Msg> {
        self.map_vec_into(|msg| msg.for_file(file, mod_start))
    }

    fn in_mod(self, m: &impl SpanFile) -> Vec<Msg> {
        self.map_vec_into(|msg| msg.in_mod(m))
    }
}

impl<T> ToMsg<DiagnosticResult<T>> for DiagnosticResult<T> {
    fn for_file(self, file: usize, mod_start: Option<usize>) -> DiagnosticResult<T> {
        self.map_err(|e| e.for_file(file, mod_start))
    }

    fn in_mod(self, m: &impl SpanFile) -> DiagnosticResult<T> {
        self.map_err(|e| e.in_mod(m))
    }
}

// Convert syn::Result to ParseMsg
pub trait CatchSpanErr<T> {
    fn catch_err(self, msg: &str) -> DiagnosticResult<T>;

    fn catch_err_span(self, msg: &str, span: Span) -> DiagnosticResult<T>;
}

impl<T> CatchSpanErr<T> for syn::Result<T> {
    fn catch_err(self, msg: &str) -> DiagnosticResult<T> {
        self.map_err(|e| vec![ParseMsg::from_span(msg, e.span())])
    }

    fn catch_err_span(self, msg: &str, span: Span) -> DiagnosticResult<T> {
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
    pub fn in_mod(msg: &str, m: &impl SpanFile, span: &(impl Spanned + ?Sized)) -> Self {
        Self::from_span(msg.to_string(), m.span_file(), span.span(), m.span_start())
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

pub type DiagnosticResult<T> = msg_result::DiagnosticResult<T, Msg>;

// Inject span into string messages
pub trait InjectSpan {
    fn in_mod(self, m: &impl SpanFile, span: &(impl Spanned + ?Sized)) -> Self;

    fn in_file(self, f: usize, span: &(impl Spanned + ?Sized), start: Option<usize>) -> Self;
}

impl InjectSpan for Msg {
    fn in_mod(self, m: &impl SpanFile, span: &(impl Spanned + ?Sized)) -> Self {
        self.in_file(m.span_file(), span, m.span_start())
    }

    fn in_file(self, f: usize, span: &(impl Spanned + ?Sized), start: Option<usize>) -> Self {
        if let Self::String(msg) = &self {
            return Self::for_file(msg, f, span, start);
        }
        self
    }
}

impl InjectSpan for Vec<Msg> {
    fn in_mod(self, m: &impl SpanFile, span: &(impl Spanned + ?Sized)) -> Self {
        self.map_vec_into(|msg| msg.in_mod(m, span))
    }

    fn in_file(self, f: usize, span: &(impl Spanned + ?Sized), start: Option<usize>) -> Self {
        self.map_vec_into(|msg| msg.in_file(f, span, start))
    }
}

impl<T> InjectSpan for DiagnosticResult<T> {
    fn in_mod(self, m: &impl SpanFile, span: &(impl Spanned + ?Sized)) -> Self {
        self.map_err(|msgs| msgs.in_mod(m, span))
    }

    fn in_file(self, f: usize, span: &(impl Spanned + ?Sized), start: Option<usize>) -> Self {
        self.map_err(|msgs| msgs.in_file(f, span, start))
    }
}

// Convert error type to Msg
pub trait CatchErr<T> {
    fn catch_err(self, msg: &str) -> DiagnosticResult<T>;
}

impl<T> CatchErr<T> for Option<T> {
    fn catch_err(self, msg: &str) -> DiagnosticResult<T> {
        self.ok_or_else(|| vec![Error::new(&msg.to_string())])
    }
}

impl<T, E> CatchErr<T> for Result<T, E> {
    fn catch_err(self, msg: &str) -> DiagnosticResult<T> {
        self.map_err(|_| vec![Error::new(&msg.to_string())])
    }
}

// Access Vec or get error
pub trait GetVec<T> {
    fn try_get<'a>(&'a self, i: usize) -> DiagnosticResult<&'a T>;

    fn try_get_mut<'a>(&'a mut self, i: usize) -> DiagnosticResult<&'a mut T>;
}

impl<T> GetVec<T> for Vec<T> {
    fn try_get<'a>(&'a self, i: usize) -> DiagnosticResult<&'a T> {
        self.get(i).catch_err(&format!("Invalid index: {i}"))
    }

    fn try_get_mut<'a>(&'a mut self, i: usize) -> DiagnosticResult<&'a mut T> {
        self.get_mut(i).catch_err(&format!("Invalid index: {i}"))
    }
}

// Convert errors to compile_error!()
pub trait ToCompileErr<T> {
    fn to_compile_errors(self, span: Span) -> Result<T, TokenStream>;
}

impl<T> ToCompileErr<T> for DiagnosticResult<T> {
    fn to_compile_errors(self, span: Span) -> Result<T, TokenStream> {
        self.map_err(|errs| {
            let errs = errs.map_vec_into(|err| {
                syn::Error::new(
                    span,
                    match err {
                        Msg::Diagnostic { msg, .. } => msg,
                        Error::new(&msg) => msg,
                    },
                )
                .into_compile_error()
            });
            quote!(#(#errs)*)
        })
    }
}
