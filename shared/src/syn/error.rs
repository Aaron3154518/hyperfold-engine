use backtrace::Backtrace;
use diagnostic::{CatchErr, ErrorSpan, Results, ToErr};
use syn::spanned::Spanned;

use crate::traits::CollectVecInto;

// Error when something went wrong in the code
#[derive(Debug, Clone)]
pub struct PanicError {
    pub msg: String,
    pub backtrace: Backtrace,
}

pub type PanicResult<T> = Results<T, PanicError>;

impl PanicError {
    pub fn new(msg: &str) -> Self {
        Self {
            msg: msg.to_string(),
            backtrace: Backtrace::new(),
        }
    }
}

impl From<String> for PanicError {
    fn from(value: String) -> Self {
        Self::new(&value)
    }
}

impl From<&str> for PanicError {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

pub fn panic(msg: &str) -> PanicError {
    PanicError::new(msg)
}

// Error with no span
#[derive(Debug, Clone)]
pub struct UnspannedError {
    pub msg: String,
}

pub type UnspannedResult<T> = Results<T, UnspannedError>;

impl UnspannedError {
    pub fn new(msg: &str) -> Self {
        Self {
            msg: msg.to_string(),
        }
    }
}

impl From<String> for UnspannedError {
    fn from(value: String) -> Self {
        Self::new(&value)
    }
}

impl From<&str> for UnspannedError {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

// Error with span
#[derive(Debug, Clone)]
pub struct SpannedError {
    pub msg: String,
    pub span: ErrorSpan,
}

pub type SpannedResult<T> = Results<T, SpannedError>;

impl SpannedError {
    pub fn new(msg: &str, span: impl Into<ErrorSpan>) -> Self {
        Self {
            msg: msg.to_string(),
            span: span.into(),
        }
    }
}

pub fn err(msg: &str, span: impl Into<ErrorSpan>) -> SpannedError {
    SpannedError::new(msg, span)
}

// special case for F = syn::Error
pub trait CatchSynError<T> {
    fn catch_syn_err(self, msg: &str) -> SpannedResult<T>;
}

impl<T> CatchSynError<T> for Result<T, syn::Error> {
    fn catch_syn_err(self, msg: &str) -> SpannedResult<T> {
        self.map_err(|e| err(msg, &e.span()).as_vec())
    }
}

// Error that may have a span
#[derive(Debug, Clone)]
pub enum BuildError {
    Panic(PanicError),
    Spanned(SpannedError),
}

// Collect errors into a result
pub type BuildResult<T> = Results<T, BuildError>;

impl From<PanicError> for BuildError {
    fn from(value: PanicError) -> Self {
        Self::Panic(value)
    }
}

impl<T> From<PanicError> for BuildResult<T> {
    fn from(value: PanicError) -> Self {
        Err(vec![value.into()])
    }
}

impl From<SpannedError> for BuildError {
    fn from(value: SpannedError) -> Self {
        Self::Spanned(value)
    }
}

impl<T> From<SpannedError> for BuildResult<T> {
    fn from(value: SpannedError) -> Self {
        Err(vec![value.into()])
    }
}

// Convert Into<ErrorSpan> into SpannedError
pub trait ToError {
    fn error(&self, msg: &str) -> SpannedError;
}

impl<T> ToError for T
where
    T: Spanned,
{
    fn error(&self, msg: &str) -> SpannedError {
        SpannedError::new(msg, self)
    }
}

// Convert from UnspannedResults to SpannedResult
pub trait AddSpan<T> {
    fn add_span(self, span: impl Into<ErrorSpan>) -> SpannedResult<T>;
}

impl<T> AddSpan<T> for UnspannedResult<T> {
    fn add_span(self, span: impl Into<ErrorSpan>) -> SpannedResult<T> {
        let span = span.into();
        self.map_err(|errs| errs.map_vec_into(|msg| SpannedError { msg: msg.msg, span }))
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
