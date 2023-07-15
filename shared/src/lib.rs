#![feature(pattern)]

pub mod constants;
pub mod file;
mod macro_args;
pub mod msg_result;
pub mod traits;
mod util_macros;

pub mod parsing {
    pub use crate::macro_args::{ComponentMacroArgs, GlobalMacroArgs, SystemMacroArgs};
}

pub mod macros {
    pub use crate::{hash_map, let_mut_vecs};
    pub use shared_macros::expand_enum;
}
