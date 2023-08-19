use backtrace::Backtrace;
use codespan_reporting::diagnostic::{Label, Severity};
use diagnostic::{
    CatchErr, CodespanDiagnostic, Diagnostic, DiagnosticLevel, ErrorSpan, Renderer, Results, ToErr,
};
use syn::spanned::Spanned;

use crate::traits::CollectVecInto;

#[derive(Debug, Clone)]
pub struct Error {
    pub diagnostic: CodespanDiagnostic<usize>,
    pub span: ErrorSpan,
}

pub type Result<T> = Results<T, Error>;

impl From<DiagnosticLevel> for Error {
    fn from(value: DiagnosticLevel) -> Self {
        Self {
            diagnostic: CodespanDiagnostic::<usize>::from(value),
            span: Default::default(),
        }
    }
}

impl Error {
    pub fn error() -> Self {
        Self::from(DiagnosticLevel::Error)
    }

    pub fn warning() -> Self {
        Self::from(DiagnosticLevel::Warning)
    }

    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.diagnostic.message = msg.into();
        self
    }

    pub fn with_backtrace(mut self) -> Self {
        self.diagnostic
            .notes
            .push(format!("{:#?}", Backtrace::new()));
        self
    }

    pub fn without_backtrace(mut self) -> Self {
        self.diagnostic.notes = Vec::new();
        self
    }

    pub fn with_span(mut self, span: impl Into<ErrorSpan>) -> Self {
        self.span = span.into();
        self
    }

    pub fn with_file(mut self, file_id: usize) -> Self {
        self.diagnostic.labels.push(Label::primary(
            file_id,
            self.span.byte_start..self.span.byte_end,
        ));
        self
    }

    pub fn render(self, renderer: &Renderer) -> Diagnostic {
        let Self { diagnostic, span } = self.with_file(renderer.file_idx());
        Diagnostic::from_span(
            diagnostic.message.to_string(),
            renderer.file_name(),
            match diagnostic.severity {
                Severity::Error => DiagnosticLevel::Error,
                Severity::Warning | Severity::Bug => DiagnosticLevel::Warning,
                Severity::Note => DiagnosticLevel::Note,
                Severity::Help => DiagnosticLevel::Help,
            },
            Some(renderer.render(&diagnostic)),
            span,
        )
    }
}

// special case for F = syn::Error
pub trait CatchSynError<T> {
    fn catch_syn_err(self, msg: impl Into<String>) -> Result<T>;
}

impl<T> CatchSynError<T> for syn::Result<T> {
    fn catch_syn_err(self, msg: impl Into<String>) -> Result<T> {
        self.map_err(|e| e.span().error(msg).as_vec())
    }
}

// Convert String into Error
pub trait StrToError {
    fn error(self) -> Error;

    fn warning(self) -> Error;

    fn trace(self) -> Error;
}

impl<S> StrToError for S
where
    S: Into<String>,
{
    fn error(self) -> Error {
        Error::error().with_message(self)
    }

    fn warning(self) -> Error {
        Error::warning().with_message(self)
    }

    fn trace(self) -> Error {
        Error::error().with_message(self).with_backtrace()
    }
}

// Convert Spanned into Error
pub trait ToError {
    fn error(&self, msg: impl Into<String>) -> Error;

    fn warning(&self, msg: impl Into<String>) -> Error;
}

impl<T> ToError for T
where
    T: Spanned,
{
    fn error(&self, msg: impl Into<String>) -> Error {
        msg.error().with_span(self)
    }

    fn warning(&self, msg: impl Into<String>) -> Error {
        msg.warning().with_span(self)
    }
}

// Add span to Result
pub trait MutateResults {
    fn with_span(self, span: impl Into<ErrorSpan>) -> Self;

    fn without_backtrace(self) -> Self;
}

impl<T> MutateResults for Result<T> {
    fn with_span(self, span: impl Into<ErrorSpan>) -> Result<T> {
        let span = span.into();
        self.map_err(|errs| errs.map_vec_into(|err| err.with_span(span)))
    }

    fn without_backtrace(self) -> Self {
        self.map_err(|errs| errs.map_vec_into(|err| err.without_backtrace()))
    }
}

// Get element from vec or produce MsgResult
pub trait GetVec<T> {
    fn try_get<'a>(&'a self, i: usize) -> Result<&'a T>;

    fn try_get_mut<'a>(&'a mut self, i: usize) -> Result<&'a mut T>;
}

impl<T> GetVec<T> for Vec<T> {
    fn try_get<'a>(&'a self, i: usize) -> Result<&'a T> {
        let len = self.len();
        self.get(i)
            .catch_err(format!("Invalid index: {i}/{len}").trace())
    }

    fn try_get_mut<'a>(&'a mut self, i: usize) -> Result<&'a mut T> {
        let len = self.len();
        self.get_mut(i)
            .catch_err(format!("Invalid index: {i}/{len}").trace())
    }
}
