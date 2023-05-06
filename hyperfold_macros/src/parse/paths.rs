use hyperfold_shared::paths::PREFIX;

// Hardcoded global components
pub fn crate_<'a>(n: &'a str) -> [&'a str; 2] {
    return [PREFIX, n];
}
pub const EVENT: [&str; 4] = [PREFIX, "utils", "event", "Event"];
pub const RENDER_SYSTEM: [&str; 4] = [PREFIX, "framework", "render_system", "RenderSystem"];
pub const ENTITY_TRASH: [&str; 4] = [PREFIX, "ecs", "entities", "EntityTrash"];
pub const CORE_EVENT: [&str; 4] = [PREFIX, "ecs", "events", "CoreEvent"];
pub const SCREEN: [&str; 4] = [PREFIX, "framework", "render_system", "Screen"];
pub const CAMERA: [&str; 4] = [PREFIX, "framework", "render_system", "Camera"];
