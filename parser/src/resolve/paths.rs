use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    codegen2::{util::vec_to_path, Crates},
    resolve::constants::NAMESPACE,
    util::end,
};

use shared::util::{Call, JoinMap, PushInto};

use super::{path::ItemPath, util::MsgResult};

pub trait ExpandEnum<const N: usize>
where
    Self: Sized,
{
    const LEN: usize = N;
    const VARIANTS: [Self; N];
}

// All named crate indices
#[shared::macros::expand_enum]
pub enum Crate {
    Main,
    Engine,
    Macros,
}

pub trait GetPaths {
    fn get_crate(&self) -> Crate;

    fn get_ident(&self) -> &str;

    fn get_path(&self) -> Vec<&str>;

    fn full_path(&self) -> Vec<String> {
        [vec!["crate"], self.get_path().push_into(self.get_ident())]
            .concat()
            .map_vec(|s| s.to_string())
    }
}

// Constants for defining paths
pub const ECS: &str = "ecs";

// Paths to marker macros
#[shared::macros::expand_enum]
pub enum MacroPaths {
    Component,
    Global,
    Event,
    System,
    Components,
}

impl GetPaths for MacroPaths {
    fn get_crate(&self) -> Crate {
        match self {
            MacroPaths::Component | MacroPaths::Global | MacroPaths::Event | MacroPaths::System => {
                Crate::Macros
            }
            MacroPaths::Components => Crate::Engine,
        }
    }

    fn get_ident(&self) -> &str {
        match self {
            MacroPaths::Component => "component",
            MacroPaths::Global => "global",
            MacroPaths::Event => "event",
            MacroPaths::System => "system",
            MacroPaths::Components => "components",
        }
    }

    fn get_path(&self) -> Vec<&str> {
        match self {
            MacroPaths::Component | MacroPaths::Global | MacroPaths::Event | MacroPaths::System => {
                Vec::new()
            }
            MacroPaths::Components => Vec::new(),
        }
    }
}

// Paths to base trait definitions use in codegen
#[shared::macros::expand_enum]
pub enum EngineTraits {
    AddComponent,
    AddEvent,
}

impl GetPaths for EngineTraits {
    fn get_crate(&self) -> Crate {
        Crate::Engine
    }

    fn get_ident(&self) -> &str {
        match self {
            EngineTraits::AddComponent => "AddComponent",
            EngineTraits::AddEvent => "AddEvent",
        }
    }

    fn get_path(&self) -> Vec<&str> {
        match self {
            EngineTraits::AddComponent => vec![ECS, "components"],
            EngineTraits::AddEvent => vec![ECS, "events"],
        }
    }
}

// Paths to globals needed by codegen
#[shared::macros::expand_enum]
pub enum EngineGlobals {
    CFoo,
    EFoo,
    EntityTrash,
    Event,
    Renderer,
    Camera,
    Screen,
}

impl GetPaths for EngineGlobals {
    fn get_crate(&self) -> Crate {
        match self {
            EngineGlobals::CFoo | EngineGlobals::EFoo => Crate::Main,
            _ => Crate::Engine,
        }
    }

    fn get_ident(&self) -> &str {
        match self {
            EngineGlobals::CFoo => "CFoo",
            EngineGlobals::EFoo => "EFoo",
            EngineGlobals::EntityTrash => "EntityTrash",
            EngineGlobals::Event => "Event",
            EngineGlobals::Renderer => "Renderer",
            EngineGlobals::Camera => "Camera",
            EngineGlobals::Screen => "Screen",
        }
    }

    fn get_path(&self) -> Vec<&str> {
        match self {
            EngineGlobals::CFoo | EngineGlobals::EFoo => vec![NAMESPACE],
            EngineGlobals::EntityTrash => vec![ECS, "entities"],
            EngineGlobals::Renderer | EngineGlobals::Screen | EngineGlobals::Camera => {
                vec!["framework", "render_system"]
            }
            EngineGlobals::Event => vec!["utils", "event"],
        }
    }
}

// Paths to engine items needed by parsing
#[shared::macros::expand_enum]
pub enum EnginePaths {
    // Containers
    Container,
    Label,
    AndLabels,
    OrLabels,
    NandLabels,
    NorLabels,
    Singleton,
    // Functions
    Intersect,
    IntersectMut,
    IntersectKeys,
    GetKeys,
    // Events
    CoreUpdate,
    CoreEvents,
    CorePreRender,
    CoreRender,
    // Entities
    Entity,
    EntitySet,
    EntityMap,
    // Systems
    Entities,
    // Use statements
    SDL2,
    SDL2Image,
}

impl GetPaths for EnginePaths {
    fn get_crate(&self) -> Crate {
        Crate::Engine
    }

    fn get_ident(&self) -> &str {
        match self {
            EnginePaths::Container => "Container",
            EnginePaths::Label => "Label",
            EnginePaths::AndLabels => "AndLabels",
            EnginePaths::OrLabels => "OrLabels",
            EnginePaths::NandLabels => "NandLabels",
            EnginePaths::NorLabels => "NorLabels",
            EnginePaths::Singleton => "Singleton",
            EnginePaths::Intersect => "intersect",
            EnginePaths::IntersectMut => "intersect_mut",
            EnginePaths::IntersectKeys => "intersect_keys",
            EnginePaths::GetKeys => "get_keys",
            EnginePaths::CoreUpdate => "Update",
            EnginePaths::CoreEvents => "Events",
            EnginePaths::CorePreRender => "PreRender",
            EnginePaths::CoreRender => "Render",
            EnginePaths::Entity => "Entity",
            EnginePaths::EntitySet => "EntitySet",
            EnginePaths::EntityMap => "EntityMap",
            EnginePaths::Entities => "Entities",
            EnginePaths::SDL2 => "sdl2",
            EnginePaths::SDL2Image => "sdl2_image",
        }
    }

    fn get_path(&self) -> Vec<&str> {
        match self {
            EnginePaths::Container
            | EnginePaths::Label
            | EnginePaths::AndLabels
            | EnginePaths::OrLabels
            | EnginePaths::NandLabels
            | EnginePaths::NorLabels
            | EnginePaths::Singleton => vec![ECS, "components"],
            EnginePaths::Intersect
            | EnginePaths::IntersectMut
            | EnginePaths::IntersectKeys
            | EnginePaths::GetKeys => vec!["intersect"],
            EnginePaths::CoreUpdate
            | EnginePaths::CoreEvents
            | EnginePaths::CorePreRender
            | EnginePaths::CoreRender => {
                vec![ECS, "events", "core"]
            }
            EnginePaths::Entity | EnginePaths::EntitySet | EnginePaths::EntityMap => {
                vec![ECS, "entities"]
            }
            EnginePaths::Entities => vec![ECS, "systems"],
            EnginePaths::SDL2 | EnginePaths::SDL2Image => vec![],
        }
    }
}

// Paths to traits that appear in generated namespace
#[shared::macros::expand_enum]
pub enum NamespaceTraits {
    AddComponent,
    AddEvent,
}

impl NamespaceTraits {
    pub fn get_global(&self) -> EngineGlobals {
        match self {
            NamespaceTraits::AddComponent => EngineGlobals::CFoo,
            NamespaceTraits::AddEvent => EngineGlobals::EFoo,
        }
    }
}

impl GetPaths for NamespaceTraits {
    fn get_crate(&self) -> Crate {
        Crate::Engine
    }

    fn get_ident(&self) -> &str {
        match self {
            NamespaceTraits::AddComponent => EngineTraits::AddComponent.get_ident(),
            NamespaceTraits::AddEvent => EngineTraits::AddEvent.get_ident(),
        }
    }

    fn get_path(&self) -> Vec<&str> {
        vec![NAMESPACE]
    }
}
