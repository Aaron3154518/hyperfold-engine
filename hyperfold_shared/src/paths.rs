// Hardcoded struct paths
pub const PREFIX: &str = "hyperfold_engine";

pub const ENTITY_PATH: [&str; 4] = [PREFIX, "ecs", "entities", "Entity"];
pub const COMPONENTS_PATH: [&str; 4] = [PREFIX, "ecs", "components", "Components"];

// Label paths
pub const LABEL_PATH: [&str; 4] = [PREFIX, "ecs", "components", "Label"];
pub const AND_LABELS_PATH: [&str; 4] = [PREFIX, "ecs", "components", "AndLabels"];
pub const OR_LABELS_PATH: [&str; 4] = [PREFIX, "ecs", "components", "OrLabels"];
pub const NAND_LABELS_PATH: [&str; 4] = [PREFIX, "ecs", "components", "NandLabels"];
pub const NOR_LABELS_PATH: [&str; 4] = [PREFIX, "ecs", "components", "NorLabels"];

// Global manager
pub const SYSTEMS_MANAGER: &str = "SFoo";
pub const COMPONENTS_MANAGER: &str = "CFoo";
pub const GLOBALS_MANAGER: &str = "GFoo";
pub const EVENTS_MANAGER: &str = "EFoo";
pub const COMPONENTS_TRAIT: &str = "CFooT";
pub const EVENTS_TRAIT: &str = "EFooT";
