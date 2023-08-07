#![feature(lazy_cell)]

mod diagnostic;
mod span;
mod writer;

use std::{fs, ops::Range};

use codespan_reporting::{
    diagnostic::Label,
    files::SimpleFiles,
    term::{self, Config},
};
use proc_macro2::LineColumn;

use span::ToRange;
use syn::spanned::Spanned;

use crate::writer::{Writer, WriterTrait};

pub use codespan_reporting::diagnostic::Diagnostic as CodespanDiagnostic;
pub use diagnostic::*;

impl Diagnostic {
    pub fn new(
        message: String,
        file_name: String,
        byte_start: usize,
        byte_end: usize,
        line_start: usize,
        line_end: usize,
        column_start: usize,
        column_end: usize,
    ) -> Self {
        let mut writer = Writer::empty();
        let mut files = SimpleFiles::new();
        let id = files.add(
            &file_name,
            fs::read_to_string(&file_name)
                .unwrap_or_else(|e| format!("Could not read '{file_name}': {e}")),
        );

        let config = Config::default();
        let diagnostic = CodespanDiagnostic::error()
            .with_message(message.to_string())
            .with_notes(vec!["Note1".to_string(), "Note2".to_string()])
            .with_labels(vec![Label::primary(id, byte_start..byte_end)]);
        let rendered = Some(
            term::emit(&mut writer, &config, &files, &diagnostic)
                .map(|_| writer.to_string())
                .unwrap_or_else(|e| format!("Could not produce error message: {e}")),
        );

        Self {
            message,
            code: None,
            level: DiagnosticLevel::Error,
            spans: vec![DiagnosticSpan {
                // Need
                file_name,
                byte_start: 0,
                byte_end: 0,
                // Need
                line_start,
                line_end,
                // Need
                column_start,
                column_end,
                is_primary: true,
                text: vec![],
                label: None,
                suggested_replacement: None,
                suggestion_applicability: None,
                expansion: None,
            }],
            children: vec![],
            rendered,
        }
    }

    pub fn from_span(message: String, file_name: String, span: impl Spanned) -> Self {
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

        Self::new(
            message,
            file_name,
            byte_start,
            byte_end,
            line_start,
            line_end,
            column_start,
            column_end,
        )
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    pub fn emit(&self) -> serde_json::Result<()> {
        self.to_json().map(|msg| println!("cargo:warning={msg}"))
    }
}
