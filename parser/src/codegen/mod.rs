mod codegen;
mod components;
mod crate_paths;
mod events;
mod globals;
mod manager;
mod traits;

pub use codegen::codegen;
pub use components::{component_trait_defs, component_trait_impls, components};
pub use crate_paths::Crates;
pub use events::{event_trait_defs, event_trait_impls, events, events_enum};
pub use globals::globals;
pub use manager::{manager_def, manager_impl};
pub use traits::Traits;
