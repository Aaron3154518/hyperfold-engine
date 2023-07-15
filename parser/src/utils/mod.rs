pub mod idents;
pub mod syn;

pub mod constants {
    pub const NAMESPACE: &str = "_engine";
}

// Crate index, item index
pub type ItemIndex = (usize, usize);
