pub mod event;
pub mod mouse;

pub mod events {
    pub use super::event::{Key, Mouse};
    pub use super::mouse::{Click, Drag, DragEnd, DragStart};
}

pub mod components {
    pub use super::mouse::DragTrigger;
}

pub mod globals {
    pub use super::mouse::DragState;
}

pub mod sytems {
    pub use super::event::on_event;
    pub use super::mouse::{drag, mouse_event};
}

pub mod component_sets {
    pub use super::mouse::MouseArgs;
}
