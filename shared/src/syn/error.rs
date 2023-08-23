use std::{
    collections::{HashMap, HashSet},
    fs, io,
    iter::once,
};

use backtrace::Backtrace;
use codespan_reporting::{diagnostic::Label, files::SimpleFiles};
use diagnostic::{
    CatchErr, CodespanDiagnostic, Diagnostic, DiagnosticLevel, ErrorNote, ErrorSpan, ToErr,
};
use syn::spanned::Spanned;

use crate::traits::CollectVec;

// ------- STRUCTS -------
// Represents a file that has been loaded into the Renderer
struct File {
    id: usize,
    name: String,
    start_span: ErrorSpan,
    success: io::Result<()>,
}

// Contains data from the result of rendering a diagnostic
pub struct RenderResult {
    text: String,
    file: String,
    span: ErrorSpan,
}

// Renderer handles loading files, adjusting spans, and generating diagnostic text
pub struct Renderer {
    file_list: SimpleFiles<String, String>,
    files: HashMap<Option<(usize, usize)>, File>,
    default_file: String,
}

impl Renderer {
    pub fn new(def_file: impl Into<String>, def_start_span: impl Into<ErrorSpan>) -> Self {
        let mut s = Self {
            file_list: SimpleFiles::new(),
            files: HashMap::new(),
            default_file: def_file.into(),
        };
        s.add_file_impl(None, s.default_file.to_string(), def_start_span.into());
        s
    }

    pub fn add_file(
        &mut self,
        cr_idx: usize,
        m_idx: usize,
        file: impl Into<String>,
        start_span: impl Into<ErrorSpan>,
    ) {
        self.add_file_impl(Some((cr_idx, m_idx)), file.into(), start_span.into())
    }

    fn add_file_impl(&mut self, idx: Option<(usize, usize)>, file: String, start_span: ErrorSpan) {
        self.files.insert(idx, {
            let mut success = Ok(());
            File {
                id: self.file_list.add(
                    file.to_string(),
                    fs::read_to_string(file.to_string()).unwrap_or_else(|e| {
                        success = Err(e);
                        String::new()
                    }),
                ),
                name: file,
                start_span,
                success,
            }
        });
    }

    pub fn render<D>(&self, diagnostic: &D) -> RenderResult
    where
        D: SpanTrait + DiagnosticTrait,
    {
        self.render_at(diagnostic.diagnostic(), *diagnostic.span())
    }

    pub fn render_at(&self, mut diagnostic: CodespanDiagnostic<usize>, span: Span) -> RenderResult {
        const HEADER: &str = "Diagnostic Error";

        // Default span is first byte of the default file
        let idx = span.cr_idx.zip(span.m_idx);
        let mut span = span.span.unwrap_or_default();

        // Attempt to generate the primary label for this diagnostic
        let file = match self.files.get(&idx) {
            Some(f) => {
                span.subtract_bytes(f.start_span.byte_start);
                diagnostic
                    .labels
                    .push(Label::primary(f.id, span.byte_range()));
                if let Err(e) = &f.success {
                    diagnostic
                        .notes
                        .push(format!("{HEADER}: Failed to load file: {e}"))
                }
                f.name.to_string()
            }
            None => {
                diagnostic.notes.push(format!(
                    "{HEADER}: The file for {} in {} has not been loaded",
                    idx.map_or("default mod".to_string(), |(_, m)| format!("mod {m}")),
                    idx.map_or("default crate".to_string(), |(c, _)| format!("crate {c}")),
                ));
                span = Default::default();
                self.default_file.to_string()
            }
        };
        RenderResult {
            text: diagnostic::render(&self.file_list, &diagnostic)
                .unwrap_or_else(|e| format!("{HEADER}: Failed to render diagnostic: {e}")),
            file,
            span,
        }
    }
}

// Trait for get diagnostic
pub trait DiagnosticTrait {
    fn diagnostic(&self) -> CodespanDiagnostic<usize>;
}

// Data and trait for locating an error in a file
pub trait SpanTrait
where
    Self: Sized,
{
    fn span(&self) -> &Span;

    fn span_mut(&mut self) -> &mut Span;

    fn set_span(&mut self, span: impl Into<ErrorSpan>) {
        self.span_mut().span = Some(span.into());
    }

    fn set_mod(&mut self, cr_idx: usize, m_idx: usize) {
        let span = self.span_mut();
        span.cr_idx = Some(cr_idx);
        span.m_idx = Some(m_idx);
    }
}

#[derive(Copy, Clone)]
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
    fn span(&self) -> &Span {
        self
    }

    fn span_mut(&mut self) -> &mut Span {
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
    fn span(&self) -> &Span {
        &self.span
    }

    fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }
}

impl DiagnosticTrait for Note {
    fn diagnostic(&self) -> CodespanDiagnostic<usize> {
        CodespanDiagnostic::note().with_message(self.msg.to_string())
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

    pub fn render(&self, renderer: &Renderer) -> Diagnostic {
        let (diagnostic_notes, error_notes) = self.notes.unzip_vec(|note| {
            let RenderResult { text, file, span } = renderer.render(note);
            let msg = note.msg.to_string();
            (text, ErrorNote { span, msg, file })
        });
        let RenderResult { text, file, span } =
            renderer.render_at(self.diagnostic().with_notes(diagnostic_notes), *self.span());
        Diagnostic::from_span(self.msg.to_string(), file, self.level, Some(text), span)
            .with_notes(error_notes)
    }
}

impl SpanTrait for Error {
    fn span(&self) -> &Span {
        &self.span
    }

    fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }
}

impl DiagnosticTrait for Error {
    fn diagnostic(&self) -> CodespanDiagnostic<usize> {
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
}

pub type CriticalResult<T> = diagnostic::CriticalResult<T, Error>;
pub type WarningResult<T> = diagnostic::WarningResult<T, Error>;
pub type Result<T> = diagnostic::Result<T, Error>;

// ------- TRAITS -------
// special case for F = syn::Error
pub trait CatchSynError<T> {
    fn catch_syn_err(self, msg: impl Into<String>) -> CriticalResult<T>;
}

impl<T> CatchSynError<T> for syn::Result<T> {
    fn catch_syn_err(self, msg: impl Into<String>) -> CriticalResult<T> {
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

    fn with_mod(self, cr_idx: usize, m_idx: usize) -> Self;
}

impl<T> MutateResults for CriticalResult<T> {
    fn with_span(self, span: impl Into<ErrorSpan>) -> CriticalResult<T> {
        self.map_err(|errs| errs.with_span(span))
    }

    fn with_mod(self, cr_idx: usize, m_idx: usize) -> Self {
        self.map_err(|errs| errs.with_mod(cr_idx, m_idx))
    }
}

impl MutateResults for Vec<Error> {
    fn with_span(mut self, span: impl Into<ErrorSpan>) -> Self {
        let span = span.into();
        for err in &mut self {
            err.set_span(span);
        }
        self
    }

    fn with_mod(mut self, cr_idx: usize, m_idx: usize) -> Self {
        for err in &mut self {
            err.set_mod(cr_idx, m_idx);
        }
        self
    }
}

impl<T> MutateResults for T
where
    T: SpanTrait,
{
    fn with_span(mut self, span: impl Into<ErrorSpan>) -> Self {
        self.set_span(span);
        self
    }

    fn with_mod(mut self, cr_idx: usize, m_idx: usize) -> Self {
        self.set_mod(cr_idx, m_idx);
        self
    }
}

// Get element from vec or produce CriticalResult
pub trait GetVec<T> {
    fn try_get<'a>(&'a self, i: usize) -> CriticalResult<&'a T>;

    fn try_get_mut<'a>(&'a mut self, i: usize) -> CriticalResult<&'a mut T>;
}

impl<T> GetVec<T> for Vec<T> {
    fn try_get<'a>(&'a self, i: usize) -> CriticalResult<&'a T> {
        let len = self.len();
        self.get(i)
            .catch_err(format!("Invalid index: {i}/{len}").trace())
    }

    fn try_get_mut<'a>(&'a mut self, i: usize) -> CriticalResult<&'a mut T> {
        let len = self.len();
        self.get_mut(i)
            .catch_err(format!("Invalid index: {i}/{len}").trace())
    }
}
