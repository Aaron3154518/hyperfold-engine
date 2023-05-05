// Hardcoded global components
pub fn crate_<'a>(n: &'a str) -> [&'a str; 2] {
    return ["crate", n];
}
pub const EVENT: [&str; 4] = ["crate", "utils", "event", "Event"];
pub const RENDER_SYSTEM: [&str; 3] = ["crate", "asset_manager", "RenderSystem"];
pub const ENTITY_TRASH: [&str; 4] = ["crate", "ecs", "entity", "EntityTrash"];
pub const CORE_EVENT: [&str; 4] = ["crate", "ecs", "event", "CoreEvent"];
pub const RECT: [&str; 4] = ["crate", "utils", "rect", "Rect"];
pub const DIMENSIONS: [&str; 4] = ["crate", "utils", "rect", "Dimensions"];
