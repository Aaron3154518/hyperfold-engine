use std::{
    collections::{HashMap, HashSet},
    fs, io,
    iter::once,
};

use backtrace::Backtrace;
use codespan_reporting::{
    diagnostic::{Label, Severity},
    files::SimpleFiles,
    term::{self, Config},
};
use diagnostic::{
    CatchErr, CodespanDiagnostic, Diagnostic, DiagnosticLevel, ErrorNote, ErrorSpan, Results, ToErr,
};
use syn::spanned::Spanned;

use crate::traits::CollectVecInto;

struct File {
    id: usize,
    name: String,
    start_span: ErrorSpan,
    success: io::Result<()>,
}

pub struct Renderer {
    file_list: SimpleFiles<String, String>,
    files: HashMap<(usize, usize), File>,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            file_list: SimpleFiles::new(),
            files: HashMap::new(),
        }
    }

    pub fn add_file(
        &mut self,
        cr_idx: usize,
        m_idx: usize,
        file: &str,
        start_span: impl Into<ErrorSpan>,
    ) -> io::Result<()> {
        self.files.insert((cr_idx, m_idx), {
            let mut success = Ok(());
            File {
                id: self.file_list.add(
                    file.to_string(),
                    fs::read_to_string(file).unwrap_or_else(|e| {
                        success = Err(e);
                        String::new()
                    }),
                ),
                name: file.to_string(),
                start_span: start_span.into(),
                success,
            }
        });
        Ok(())
    }

    pub fn render(&self, error: &Error) -> String {
        format!(
            "{}{}",
            match &self.success {
                Ok(_) => String::new(),
                Err(e) => format!("Could not product error message: {e}"),
            },
            diagnostic::render(&self.files, &diagnostic)
                .unwrap_or_else(|e| format!("Could not produce error message: {e}"))
        )
    }
}

// Data and trait for locating an error in a file
pub trait SpanTrait
where
    Self: Sized,
{
    fn span(&mut self) -> &mut Span;

    fn with_span(mut self, span: impl Into<ErrorSpan>) -> Self {
        self.span().span = Some(span.into());
        self
    }

    fn with_mod(mut self, cr_idx: usize, m_idx: usize) -> Self {
        let span = self.span();
        span.cr_idx = Some(cr_idx);
        span.m_idx = Some(m_idx);
        self
    }
}

pub struct Span {
    pub span: Option<ErrorSpan>,
    pub cr_idx: Option<usize>,
    pub m_idx: Option<usize>,
}

impl Span {
    pub fn new() -> Self {
        Self {
            span: None,
            cr_idx: None,
            m_idx: None,
        }
    }
}

impl SpanTrait for Span {
    fn span(&mut self) -> &mut Span {
        self
    }
}

// Note to add information to an Error
pub struct Note {
    pub msg: String,
    pub span: Span,
}

impl Note {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            span: Span::new(),
        }
    }
}

impl SpanTrait for Note {
    fn span(&mut self) -> &mut Span {
        &mut self.span
    }
}

// Contains data to render an error (or warning)
pub struct Error {
    pub level: DiagnosticLevel,
    pub msg: String,
    pub notes: Vec<Note>,
    pub backtrace: Option<Note>,
    pub span: Span,
}

impl Error {
    // New
    fn new(level: DiagnosticLevel) -> Self {
        Self {
            level,
            msg: String::new(),
            notes: Vec::new(),
            backtrace: None,
            span: Span::new(),
        }
    }

    pub fn error() -> Self {
        Self::new(DiagnosticLevel::Error)
    }

    pub fn warning() -> Self {
        Self::new(DiagnosticLevel::Warning)
    }

    // Edit existing
    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.msg = msg.into();
        self
    }

    pub fn with_note(mut self, note: Note) -> Self {
        self.notes.push(note);
        self
    }

    pub fn with_notes(mut self, notes: impl IntoIterator<Item = Note>) -> Self {
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

    // Getters
    pub fn message(&self) -> String {
        self.msg.to_string()
    }

    pub fn get_diagnostic(&self) -> CodespanDiagnostic<usize> {
        match self.level {
            DiagnosticLevel::Error | DiagnosticLevel::Ice | DiagnosticLevel::FailureNote => {
                CodespanDiagnostic::error()
            }
            DiagnosticLevel::Warning => CodespanDiagnostic::warning(),
            DiagnosticLevel::Note => CodespanDiagnostic::note(),
            DiagnosticLevel::Help => CodespanDiagnostic::help(),
        }
        .with_message(&self.msg)
    }

    pub fn get_notes(&self) -> impl Iterator<Item = &Note> {
        self.notes.iter().chain(self.backtrace.iter())
    }

    fn iter_spans(&self) -> impl Iterator<Item = &Span> {
        once(&self.span).chain(self.notes.iter().map(|n| &n.span))
    }

    pub fn get_files(&self) -> HashSet<(usize, usize)> {
        self.iter_spans()
            .filter_map(|span| span.cr_idx.zip(span.m_idx))
            .collect()
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

impl SpanTrait for Error {
    fn span(&mut self) -> &mut Span {
        &mut self.span
    }
}

pub type Result<T> = Results<T, Error>;

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

    fn note(self) -> Note;
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

    fn note(self) -> Note {
        Note::new(self)
    }
}

// Convert Spanned into Error
pub trait ToError {
    fn error(&self, msg: impl Into<String>) -> Error;

    fn warning(&self, msg: impl Into<String>) -> Error;

    fn note(&self, msg: impl Into<String>) -> Note;
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

    fn note(&self, msg: impl Into<String>) -> Note {
        msg.note().with_span(self)
    }
}

// Add span to Result
pub trait MutateResults {
    fn with_span(self, span: impl Into<ErrorSpan>) -> Self;
}

impl<T> MutateResults for Result<T> {
    fn with_span(self, span: impl Into<ErrorSpan>) -> Result<T> {
        let span = span.into();
        self.map_err(|errs| errs.map_vec_into(|err| err.with_span(span)))
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
