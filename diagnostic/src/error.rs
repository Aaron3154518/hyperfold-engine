use std::ops::Range;

use proc_macro2::LineColumn;
use syn::spanned::Spanned;

use crate::span::ToRange;

#[derive(Debug, Copy, Clone)]
pub struct ErrorSpan {
    /// The byte offset in the file where this span starts from.
    pub byte_start: usize,
    /// The byte offset in the file where this span ends.
    pub byte_end: usize,
    /// 1-based. The line in the file.
    pub line_start: usize,
    /// 1-based. The line in the file.
    pub line_end: usize,
    /// 0-based, character offset.
    pub column_start: usize,
    /// 0-based, character offset.
    pub column_end: usize,
}

impl ErrorSpan {
    pub fn subtract_bytes(&mut self, off: usize) {
        let off = off.min(self.byte_start).min(self.byte_end);
        self.byte_start -= off;
        self.byte_end -= off;
    }
}

impl Default for ErrorSpan {
    fn default() -> Self {
        Self {
            byte_start: 0,
            byte_end: 1,
            line_start: 1,
            line_end: 1,
            column_start: 1,
            column_end: 2,
        }
    }
}

impl From<&ErrorSpan> for ErrorSpan {
    fn from(value: &ErrorSpan) -> Self {
        value.clone()
    }
}

impl<S> From<&S> for ErrorSpan
where
    S: Spanned,
{
    fn from(span: &S) -> Self {
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

#[derive(Debug, Clone)]
pub struct ErrorNote {
    pub span: ErrorSpan,
    pub msg: String,
    pub file: String,
}
