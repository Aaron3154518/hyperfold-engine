pub mod functions;
mod idents;
pub mod syn;

pub mod constants {
    pub use super::idents::{CodegenIdents, CODEGEN_IDENTS};
    pub const TAB: &str = "  ";
}

// Crate index, item index
pub type ItemIndex = (usize, usize);
