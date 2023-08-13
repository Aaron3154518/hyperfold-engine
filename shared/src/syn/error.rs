use diagnostic::{CatchErr, ErrorSpan, Results};
use syn::spanned::Spanned;

use crate::traits::CollectVecInto;

pub type MsgResult<T> = Results<T, String>;

// Error with span
#[derive(Debug, Clone)]
pub struct SpannedError {
    pub msg: String,
    pub span: ErrorSpan,
}

impl SpannedError {
    pub fn new(msg: &str, span: impl Into<ErrorSpan>) -> Self {
        Self {
            msg: msg.to_string(),
            span: span.into(),
        }
    }
}

pub type SpannedResult<T> = Results<T, SpannedError>;

// Error that may have a span
#[derive(Debug, Clone)]
pub struct BuildError {
    pub msg: String,
    pub span: Option<ErrorSpan>,
}

// Collect errors into a result
pub type BuildResult<T> = Results<T, BuildError>;

impl BuildError {
    pub fn new(msg: &str) -> Self {
        Self {
            msg: msg.to_string(),
            span: None,
        }
    }

    pub fn span(mut self, span: impl Into<ErrorSpan>) -> Self {
        self.span = Some(span.into());
        self
    }
}

impl From<String> for BuildError {
    fn from(value: String) -> Self {
        Self {
            msg: value,
            span: None,
        }
    }
}

impl From<SpannedError> for BuildError {
    fn from(value: SpannedError) -> Self {
        Self {
            msg: value.msg,
            span: Some(value.span),
        }
    }
}

impl<T> From<SpannedError> for BuildResult<T> {
    fn from(value: SpannedError) -> Self {
        Err(vec![value.into()])
    }
}

pub fn err(msg: &str) -> BuildError {
    BuildError::new(msg)
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

// Convert from Msg/SpannedResult to BuildResult
pub trait AsBuildResult<T> {
    fn upcast(self) -> BuildResult<T>;
}

impl<T> AsBuildResult<T> for MsgResult<T> {
    fn upcast(self) -> BuildResult<T> {
        self.map_err(|e| e.map_vec_into(|e| e.into()))
    }
}

impl<T> AsBuildResult<T> for SpannedResult<T> {
    fn upcast(self) -> BuildResult<T> {
        self.map_err(|e| e.map_vec_into(|e| e.into()))
    }
}

// Convert from MsgResult to SpannedResult
pub trait AddSpan<T> {
    fn add_span(self, span: impl Into<ErrorSpan>) -> SpannedResult<T>;
}

impl<T> AddSpan<T> for MsgResult<T> {
    fn add_span(self, span: impl Into<ErrorSpan>) -> SpannedResult<T> {
        let span = span.into();
        self.map_err(|errs| errs.map_vec_into(|msg| SpannedError { msg, span }))
    }
}

// Split BuildResults into types
pub trait SplitBuildResult<T> {
    fn split_errs(self) -> Result<T, (Vec<String>, Vec<SpannedError>)>;

    fn take_spanned_errs(self, errs: &mut Vec<SpannedError>) -> MsgResult<T>;

    fn take_msg_errs(self, errs: &mut Vec<String>) -> SpannedResult<T>;
}

impl<T> SplitBuildResult<T> for BuildResult<T> {
    fn split_errs(self) -> Result<T, (Vec<String>, Vec<SpannedError>)> {
        self.map_err(|errs| {
            let (mut msgs, mut spanned) = (Vec::new(), Vec::new());
            for BuildError { msg, span } in errs {
                match span {
                    Some(span) => spanned.push(SpannedError { msg, span }),
                    None => msgs.push(msg),
                }
            }
            (msgs, spanned)
        })
    }

    fn take_spanned_errs(self, errs: &mut Vec<SpannedError>) -> MsgResult<T> {
        self.split_errs().map_err(|(other, es)| {
            errs.extend(es);
            other
        })
    }

    fn take_msg_errs(self, errs: &mut Vec<String>) -> SpannedResult<T> {
        self.split_errs().map_err(|(es, other)| {
            errs.extend(es);
            other
        })
    }
}

// Get element from vec or produce MsgResult
pub trait GetVec<T> {
    fn try_get<'a>(&'a self, i: usize) -> MsgResult<&'a T>;

    fn try_get_mut<'a>(&'a mut self, i: usize) -> MsgResult<&'a mut T>;
}

impl<T> GetVec<T> for Vec<T> {
    fn try_get<'a>(&'a self, i: usize) -> MsgResult<&'a T> {
        self.get(i).catch_err(format!("Invalid index: {i}"))
    }

    fn try_get_mut<'a>(&'a mut self, i: usize) -> MsgResult<&'a mut T> {
        self.get_mut(i).catch_err(format!("Invalid index: {i}"))
    }
}
