use std::ops::Range;

use proc_macro2::LineColumn;
use syn::spanned::Spanned;

use crate::{span::ToRange, DiagnosticResult};

#[derive(Debug, Copy, Clone)]
pub struct ErrorSpan {
    /// The byte offset in the file where this span starts from.
    pub byte_start: usize,
    /// The byte offset in the file where this span ends.
    pub byte_end: usize,
    /// 0-based. The line in the file.
    pub line_start: usize,
    /// 0-based. The line in the file.
    pub line_end: usize,
    /// 0-based, character offset.
    pub column_start: usize,
    /// 0-based, character offset.
    pub column_end: usize,
}

impl<S> From<S> for ErrorSpan
where
    S: Spanned,
{
    fn from(span: S) -> Self {
        let span = span.span();

        let LineColumn {
            line: line_start,
            column: column_start,
        } = span.start();
        let LineColumn {
            line: line_end,
            column: column_end,
        } = span.end();
        let Range {
            start: byte_start,
            end: byte_end,
        } = span.to_range().unwrap_or(0..0);

        Self {
            byte_start,
            byte_end,
            line_start,
            line_end,
            column_start,
            column_end,
        }
    }
}

pub struct SpannedError {
    pub msg: String,
    pub file: String,
    pub span: ErrorSpan,
}

pub enum Error {
    Spanned(SpannedError),
    Message { msg: String },
}

impl Error {
    pub fn new(msg: &str) -> Self {
        Self::Message {
            msg: msg.to_string(),
        }
    }

    pub fn spanned(msg: &str, file: &str, span: impl Into<ErrorSpan>) -> Self {
        Self::Spanned(SpannedError {
            msg: msg.to_string(),
            file: file.to_string(),
            span: span.into(),
        })
    }

    pub fn set_span(&mut self, new_file: &str, new_span: impl Into<ErrorSpan>) {
        match self {
            Error::Spanned(SpannedError { file, span, .. }) => {
                *file = new_file.to_string();
                *span = new_span.into();
            }
            Error::Message { msg } => *self = Self::spanned(msg, new_file, new_span),
        }
    }
}

// Create new error from message and object
pub trait NewError {
    fn error<T>(self, msg: &str, file: &str) -> DiagnosticResult<T>;
}

impl<S> NewError for S
where
    S: Into<ErrorSpan>,
{
    fn error<T>(self, msg: &str, file: &str) -> DiagnosticResult<T> {
        Err(vec![Error::Spanned(SpannedError {
            msg: msg.to_string(),
            file: file.to_string(),
            span: self.into(),
        })])
    }
}

// Convert Option/Result to DiagnosticResult
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

// // Convert syn::Result to Error
// pub trait CatchSpanErr<T> {
//     fn catch_err(self, msg: &str) -> DiagnosticResult<T>;

//     fn catch_err_span(self, msg: &str, span: Span) -> DiagnosticResult<T>;
// }

// impl<T> CatchSpanErr<T> for syn::Result<T> {
//     fn catch_err(self, msg: &str) -> DiagnosticResult<T> {
//         self.map_err(|e| vec![Error::spanned(msg, "", e.span())])
//     }

//     fn catch_err_span(self, msg: &str, span: Span) -> DiagnosticResult<T> {
//         self.map_err(|_| vec![Error::spanned(msg, "", span)])
//     }
// }
