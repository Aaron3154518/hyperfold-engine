#![feature(lazy_cell)]

mod diagnostic;
mod error;
mod result;
mod span;
mod writer;

use std::fs;

use codespan_reporting::{
    files::SimpleFiles,
    term::{self, Config},
};

use crate::writer::{Writer, WriterTrait};

pub use codespan_reporting::diagnostic::Diagnostic as CodespanDiagnostic;
pub use diagnostic::*;
pub use error::*;
pub use result::*;

pub struct Renderer {
    files: SimpleFiles<String, String>,
    file_name: String,
    file_idx: usize,
}

impl Renderer {
    pub fn new(file: &str) -> Self {
        let mut files = SimpleFiles::new();
        Self {
            file_name: file.to_string(),
            file_idx: files.add(
                file.to_string(),
                fs::read_to_string(file)
                    .unwrap_or_else(|e| format!("Could not read '{file}': {e}")),
            ),
            files,
        }
    }

    pub fn file_idx(&self) -> usize {
        self.file_idx
    }

    pub fn file_name(&self) -> String {
        self.file_name.to_string()
    }

    pub fn render(&self, diagnostic: &CodespanDiagnostic<usize>) -> String {
        let mut writer = Writer::empty();
        let config = Config::default();
        term::emit(&mut writer, &config, &self.files, diagnostic)
            .map(|_| writer.to_string())
            .unwrap_or_else(|e| format!("Could not produce error message: {e}"))
    }
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
            line_start + 1,
            line_end + 1,
            column_start + 1,
            column_end + 1,
        )
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    pub fn emit(&self) -> serde_json::Result<()> {
        self.to_json().map(|msg| println!("cargo:warning={msg}"))
    }
}
