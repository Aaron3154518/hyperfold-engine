pub mod functions;
mod idents;
pub mod syn;

pub mod constants {
    pub use super::idents::{CodegenIdents, CODEGEN_IDENTS};
}

// Crate index, item index
pub type ItemIndex = (usize, usize);
