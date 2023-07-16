use codespan_reporting::files::SimpleFiles;
use once_cell::sync::Lazy;

pub mod idents;
mod msg;
pub mod paths;
pub mod syn;

pub mod constants {
    pub const NAMESPACE: &str = "_engine";
}

// Crate index, item index
pub type ItemIndex = (usize, usize);

pub type SpanFiles = SimpleFiles<String, String>;

pub use msg::{Msg, MsgResult};
