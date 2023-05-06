// Hardcoded global components
pub fn crate_<'a>(n: &'a str) -> [&'a str; 2] {
    return ["crate", n];
}
pub const EVENT: [&str; 4] = ["crate", "utils", "event", "Event"];
pub const RENDER_SYSTEM: [&str; 4] = ["crate", "framework", "render_system", "RenderSystem"];
pub const ENTITY_TRASH: [&str; 4] = ["crate", "ecs", "entity", "EntityTrash"];
pub const CORE_EVENT: [&str; 4] = ["crate", "ecs", "event", "CoreEvent"];
pub const SCREEN: [&str; 4] = ["crate", "framework", "render_system", "Screen"];
pub const CAMERA: [&str; 4] = ["crate", "framework", "render_system", "Camera"];