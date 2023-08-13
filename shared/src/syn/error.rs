use diagnostic::{ErrorSpan, Results, ToErr};
use syn::spanned::Spanned;

// Error with no span
pub type Error = String;

// Error with span
pub struct SpannedError {
    pub msg: String,
    pub span: ErrorSpan,
}

// Error that may have a span
pub struct BuildError {
    pub msg: String,
    pub span: Option<ErrorSpan>,
}

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

pub fn err(msg: &str) -> BuildError {
    BuildError::new(msg)
}

// Convert Into<ErrorSpan> into BuildError
pub trait ToError {
    fn error(&self, msg: &str) -> BuildError;
}

impl<T> ToError for T
where
    T: Spanned,
{
    fn error(&self, msg: &str) -> BuildError {
        BuildError::new(msg).span(self)
    }
}

// Collect errors into a result
pub type BuildResult<T> = Results<T, BuildError>;

// Convert error to BuildError
pub trait CatchErr<T> {
    fn catch_err(self, err: BuildError) -> BuildResult<T>;
}

impl<T, E> CatchErr<T> for Result<T, E> {
    fn catch_err(self, err: BuildError) -> BuildResult<T> {
        self.map_err(|_| err.vec())
    }
}
