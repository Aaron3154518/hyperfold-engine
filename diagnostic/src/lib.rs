#![feature(lazy_cell)]

mod critical_result;
mod diagnostic;
mod error;
mod result;
mod span;
mod warning_result;
mod writer;

use codespan_reporting::{
    files::SimpleFiles,
    term::{self, Config},
};

use crate::writer::{Writer, WriterTrait};

pub use codespan_reporting::diagnostic::Diagnostic as CodespanDiagnostic;
pub use critical_result::*;
pub use diagnostic::*;
pub use error::*;
pub use result::*;
pub use warning_result::*;

pub fn render(
    files: &SimpleFiles<String, String>,
    diagnostic: &CodespanDiagnostic<usize>,
) -> std::result::Result<String, codespan_reporting::files::Error> {
    let mut writer = Writer::empty();
    let config = Config::default();
    term::emit(&mut writer, &config, files, diagnostic).map(|_| writer.to_string())
}

impl<T> From<DiagnosticLevel> for CodespanDiagnostic<T> {
    fn from(value: DiagnosticLevel) -> Self {
        match value {
            DiagnosticLevel::Note | DiagnosticLevel::Warning | DiagnosticLevel::Help => {
                CodespanDiagnostic::warning()
            }
            DiagnosticLevel::Error | DiagnosticLevel::FailureNote | DiagnosticLevel::Ice => {
                CodespanDiagnostic::error()
            }
        }
    }
}

impl Diagnostic {
    pub fn new(
        message: String,
        file_name: String,
        level: DiagnosticLevel,
        rendered: Option<String>,
        byte_start: usize,
        byte_end: usize,
        line_start: usize,
        line_end: usize,
        column_start: usize,
        column_end: usize,
    ) -> Self {
        Self {
            message,
            code: None,
            level,
            spans: vec![DiagnosticSpan {
                // Need
                file_name,
                byte_start: byte_start as u32,
                byte_end: byte_end as u32,
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
        rendered: Option<String>,
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
            rendered,
            byte_start,
            byte_end,
            line_start,
            line_end,
            column_start + 1,
            column_end + 1,
        )
    }

    pub fn with_notes(mut self, notes: impl IntoIterator<Item = ErrorNote>) -> Self {
        self.spans.extend(
            notes
                .into_iter()
                .map(|ErrorNote { span, msg, file }| DiagnosticSpan {
                    file_name: file,
                    byte_start: span.byte_start as u32,
                    byte_end: span.byte_end as u32,
                    line_start: span.line_start,
                    line_end: span.line_end,
                    column_start: span.column_start + 1,
                    column_end: span.column_end + 1,
                    is_primary: false,
                    text: Vec::new(),
                    label: Some(msg),
                    suggested_replacement: None,
                    suggestion_applicability: None,
                    expansion: None,
                }),
        );
        self
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    pub fn emit(&self) -> serde_json::Result<()> {
        self.to_json().map(|msg| println!("cargo:warning={msg}"))
    }
}
