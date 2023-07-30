use codespan_reporting::files::SimpleFiles;
use once_cell::sync::Lazy;

mod diagnostic;
pub mod idents;
pub mod paths;
pub mod writer;

pub use diagnostic::warn;

pub mod constants {
    pub const NAMESPACE: &str = "_engine";
}

// Crate index, item index
pub type ItemIndex = (usize, usize);

pub type SpanFiles = SimpleFiles<String, String>;
