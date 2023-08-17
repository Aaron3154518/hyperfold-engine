use backtrace::Backtrace;
use codespan_reporting::diagnostic::Label;
use diagnostic::{CatchErr, CodespanDiagnostic, DiagnosticLevel, ErrorSpan, Results, ToErr};
use syn::spanned::Spanned;

#[derive(Debug, Clone)]
pub struct Error {
    pub diagnostic: CodespanDiagnostic<usize>,
    pub span: ErrorSpan,
}

pub type Result<T> = Results<T, Error>;

impl Error {
    pub fn new(msg: &str, span: ErrorSpan, level: DiagnosticLevel, backtrace: bool) -> Self {
        Self {
            diagnostic: CodespanDiagnostic::<usize>::from(level)
                .with_message(msg)
                .with_notes(
                    backtrace
                        .then(|| Backtrace::new())
                        .map_or(Vec::new(), |b| vec![format!("{b:#?}")]),
                ),
            span,
        }
    }

    pub fn with_file(&mut self, file_id: usize) -> &CodespanDiagnostic<usize> {
        self.diagnostic.labels.push(Label::primary(
            file_id,
            self.span.byte_start..self.span.byte_end,
        ));
        &self.diagnostic
    }
}

pub fn err(msg: &str, span: impl Into<ErrorSpan>) -> Error {
    Error::new(msg, span, DiagnosticLevel::Error, false)
}

pub fn warn(msg: &str, span: impl Into<ErrorSpan>) -> Error {
    Error::new(msg, span, DiagnosticLevel::Warning, false)
}

pub fn panic(msg: &str) -> Error {
    Error::new(msg, Default::default(), DiagnosticLevel::Error, true)
}

// special case for F = syn::Error
pub trait CatchSynError<T> {
    fn catch_syn_err(self, msg: &str) -> Result<T>;
}

impl<T> CatchSynError<T> for syn::Result<T> {
    fn catch_syn_err(self, msg: &str) -> Result<T> {
        self.map_err(|e| err(msg, &e.span()).as_vec())
    }
}

// Convert Into<ErrorSpan> into Error
pub trait ToError {
    fn error(&self, msg: &str) -> Error;

    fn warning(&self, msg: &str) -> Error;
}

impl<T> ToError for T
where
    T: Spanned,
{
    fn error(&self, msg: &str) -> Error {
        err(msg, self)
    }

    fn warning(&self, msg: &str) -> Error {
        warn(msg, self)
    }
}

// Add span to Result
pub trait AddSpan<T> {
    fn add_span(self, span: impl Into<ErrorSpan>) -> SpannedResult<T>;
}

impl<T> AddSpan<T> for UnspannedResult<T> {
    fn add_span(self, span: impl Into<ErrorSpan>) -> SpannedResult<T> {
        let span = span.into();
        self.map_err(|errs| errs.map_vec_into(|msg| err(&msg.msg, span)))
    }
}

// Split BuildResults into types
pub trait SplitBuildResult<T> {
    fn split_errs(self) -> Result<T, (Vec<PanicError>, Vec<SpannedError>)>;

    fn record_spanned_errs(self, errs: &mut Vec<SpannedError>) -> PanicResult<T>;

    fn record_msg_errs(self, errs: &mut Vec<PanicError>) -> SpannedResult<T>;
}

impl<T> SplitBuildResult<T> for BuildResult<T> {
    fn split_errs(self) -> Result<T, (Vec<PanicError>, Vec<SpannedError>)> {
        self.map_err(|errs| {
            let (mut panics, mut spanned) = (Vec::new(), Vec::new());
            for err in errs {
                match err {
                    BuildError::Panic(p) => panics.push(p),
                    BuildError::Spanned(s) => spanned.push(s),
                }
            }
            (panics, spanned)
        })
    }

    fn record_spanned_errs(self, errs: &mut Vec<SpannedError>) -> PanicResult<T> {
        self.split_errs().map_err(|(other, es)| {
            errs.extend(es);
            other
        })
    }

    fn record_msg_errs(self, errs: &mut Vec<PanicError>) -> SpannedResult<T> {
        self.split_errs().map_err(|(es, other)| {
            errs.extend(es);
            other
        })
    }
}

// Get element from vec or produce MsgResult
pub trait GetVec<T> {
    fn try_get<'a>(&'a self, i: usize) -> PanicResult<&'a T>;

    fn try_get_mut<'a>(&'a mut self, i: usize) -> PanicResult<&'a mut T>;
}

impl<T> GetVec<T> for Vec<T> {
    fn try_get<'a>(&'a self, i: usize) -> PanicResult<&'a T> {
        let len = self.len();
        self.get(i).catch_err(format!("Invalid index: {i}/{len}"))
    }

    fn try_get_mut<'a>(&'a mut self, i: usize) -> PanicResult<&'a mut T> {
        let len = self.len();
        self.get_mut(i)
            .catch_err(format!("Invalid index: {i}/{len}"))
    }
}
