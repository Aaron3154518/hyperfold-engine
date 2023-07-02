mod components;
mod crate_paths;
mod events;
mod globals;
mod idents;
mod systems;
mod traits;
pub mod util;

pub use components::{component_trait_defs, component_trait_impls, components};
pub use crate_paths::Crates;
pub use events::{event_trait_defs, event_trait_impls, events, events_enum};
pub use globals::globals;
pub use systems::systems;
pub use traits::Traits;

pub struct Codegen;
