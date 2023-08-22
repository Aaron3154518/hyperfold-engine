use backtrace::Backtrace;
use codespan_reporting::diagnostic::{Label, Severity};
use diagnostic::{
    CatchErr, CodespanDiagnostic, Diagnostic, DiagnosticLevel, ErrorNote, ErrorSpan, Renderer,
    Results, ToErr,
};
use syn::spanned::Spanned;

use crate::traits::{CollectVec, CollectVecInto};

#[derive(Debug, Clone)]
pub struct Error {
    pub diagnostic: CodespanDiagnostic<usize>,
    pub span: Option<ErrorSpan>,
    pub notes: Vec<(ErrorSpan, String)>,
}

pub type Result<T> = Results<T, Error>;

impl From<DiagnosticLevel> for Error {
    fn from(value: DiagnosticLevel) -> Self {
        Self {
            diagnostic: CodespanDiagnostic::<usize>::from(value),
            span: None,
            notes: Vec::new(),
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

    pub fn message(&self) -> String {
        self.diagnostic.message.to_string()
    }

    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.diagnostic.message = msg.into();
        self
    }

    pub fn with_note(mut self, span: impl Into<ErrorSpan>, msg: impl Into<String>) -> Self {
        self.notes.push((span.into(), msg.into()));
        self
    }

    pub fn with_notes(mut self, notes: impl IntoIterator<Item = (ErrorSpan, String)>) -> Self {
        self.notes.extend(notes);
        self
    }

    pub fn with_backtrace(mut self) -> Self {
        // TODO: optimize
        // self.diagnostic
        //     .notes
        //     .push(format!("{:#?}", Backtrace::new()));
        self
    }

    pub fn without_backtrace(mut self) -> Self {
        self.diagnostic.notes = Vec::new();
        self
    }

    pub fn with_span(mut self, span: impl Into<ErrorSpan>) -> Self {
        if self.span.is_none() {
            self.span = Some(span.into());
        }
        self
    }

    pub fn with_file(mut self, file_id: usize) -> Self {
        let span = self.span.unwrap_or_default();
        self.diagnostic
            .labels
            .push(Label::primary(file_id, span.byte_start..span.byte_end));
        // TODO: notes in different files
        // self.diagnostic.notes.extend(
        //     self.labels
        //         .map_vec(|(s, _)| Label::secondary(file_id, s.byte_start..s.byte_end)),
        // );
        self
    }

    pub fn subtract_span(mut self, span: &ErrorSpan) -> Self {
        if let Some(curr_span) = &mut self.span {
            curr_span.subtract_bytes(span.byte_start);
        }
        for (s, _) in &mut self.notes {
            s.subtract_bytes(span.byte_start);
        }
        self
    }

    pub fn render(self, renderer: &Renderer) -> Diagnostic {
        let file_idx = renderer.file_idx();
        let Self {
            mut diagnostic,
            span,
            notes: labels,
        } = self.with_file(file_idx);
        diagnostic.notes.extend(labels.iter().map(|(span, msg)| {
            renderer.render(
                &CodespanDiagnostic::note()
                    .with_message(msg)
                    .with_labels(vec![Label::primary(
                        file_idx,
                        span.byte_start..span.byte_end,
                    )]),
            )
        }));
        let file = renderer.file_name();
        Diagnostic::from_span(
            diagnostic.message.to_string(),
            file.to_string(),
            match diagnostic.severity {
                Severity::Error => DiagnosticLevel::Error,
                Severity::Warning | Severity::Bug => DiagnosticLevel::Warning,
                Severity::Note => DiagnosticLevel::Note,
                Severity::Help => DiagnosticLevel::Help,
            },
            Some(renderer.render(&diagnostic)),
            span.unwrap_or_default(),
        )
        .with_notes(labels.into_iter().map(|(span, msg)| ErrorNote {
            span,
            msg,
            file: file.to_string(),
        }))
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
