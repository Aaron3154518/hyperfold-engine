#![feature(lazy_cell)]

mod diagnostic;
mod error;
mod result;
mod span;
mod writer;

use std::fs;

use codespan_reporting::{
    diagnostic::Label,
    files::SimpleFiles,
    term::{self, Config},
};

use crate::writer::{Writer, WriterTrait};

pub use codespan_reporting::diagnostic::Diagnostic as CodespanDiagnostic;
pub use diagnostic::*;
pub use error::*;
pub use result::*;

impl Diagnostic {
    pub fn new(
        message: String,
        file_name: String,
        level: DiagnosticLevel,
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
        let diagnostic = match level {
            DiagnosticLevel::Note | DiagnosticLevel::Warning | DiagnosticLevel::Help => {
                CodespanDiagnostic::warning()
            }
            DiagnosticLevel::Error | DiagnosticLevel::FailureNote | DiagnosticLevel::Ice => {
                CodespanDiagnostic::error()
            }
        }
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
            level,
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

    pub fn from_span(
        message: String,
        file_name: String,
        level: DiagnosticLevel,
        span: impl Into<ErrorSpan>,
    ) -> Self {
        let ErrorSpan {
            byte_start,
            byte_end,
            line_start,
            line_end,
            column_start,
            column_end,
        } = span.into();

        Self::new(
            message,
            file_name,
            level,
            byte_start,
            byte_end,
            line_start + 1,
            line_end + 1,
            column_start + 1,
            column_end + 1,
        )
    }

    pub fn without_span(message: String, file_name: String, level: DiagnosticLevel) -> Self {
        Self::new(message, file_name, level, 0, 1, 1, 1, 1, 2)
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    pub fn emit(&self) -> serde_json::Result<()> {
        self.to_json().map(|msg| println!("cargo:warning={msg}"))
    }
}
