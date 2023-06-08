use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    codegen::{idents::Idents, util::vec_to_path},
    util::end,
    validate::constants::NAMESPACE,
};

use shared::util::{Call, JoinMap};

use super::ast_resolve::Path;

pub trait ExpandEnum<const N: usize>
where
    Self: Sized,
{
    const LEN: usize = N;
    const VARIANTS: [Self; N];
}

pub trait GetPaths<const N: usize>: ExpandEnum<N> {
    // crate
    fn as_crate(&self) -> Crates;

    // ident
    fn as_ident(&self) -> &str;

    fn to_ident(&self) -> syn::Ident {
        format_ident!("{}", self.as_ident())
    }

    // path
    fn as_path(&self) -> Vec<&str> {
        Vec::new()
    }

    fn to_path(&self) -> syn::Path {
        vec_to_path(self.path_stem())
    }

    fn construct_path(&self, path: &syn::Path) -> TokenStream {
        self.to_path().call(|i| quote!(#path::#i))
    }

    // path::ident
    fn path_stem(&self) -> Vec<String> {
        let mut v = self.as_path();
        v.push(self.as_ident());
        v.map_vec(|s| s.to_string())
    }

    // crate::path::ident
    fn full_path(&self) -> Vec<String> {
        [vec!["crate"], self.as_path(), vec![self.as_ident()]]
            .concat()
            .map_vec(|s| s.to_string())
    }

    fn crate_path(&self, cr_idx: usize) -> Path {
        Path {
            cr_idx,
            path: self.full_path(),
        }
    }

    fn crate_paths(cr_idxs: &[usize; Crates::LEN]) -> [Path; N] {
        Self::VARIANTS.map(|v| v.crate_path(cr_idxs[v.as_crate() as usize]))
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

impl GetPaths<{ Self::LEN }> for MacroPaths {
    fn as_crate(&self) -> Crates {
        match self {
            MacroPaths::Component | MacroPaths::Global | MacroPaths::Event | MacroPaths::System => {
                Crates::Macros
            }
            MacroPaths::Components => Crates::Engine,
        }
    }

    fn as_ident(&self) -> &str {
        match self {
            MacroPaths::Component => "component",
            MacroPaths::Global => "global",
            MacroPaths::Event => "event",
            MacroPaths::System => "system",
            MacroPaths::Components => "components",
        }
    }

    fn as_path(&self) -> Vec<&str> {
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

impl GetPaths<{ Self::LEN }> for EngineTraits {
    fn as_crate(&self) -> Crates {
        Crates::Engine
    }

    fn as_ident(&self) -> &str {
        match self {
            EngineTraits::AddComponent => "AddComponent",
            EngineTraits::AddEvent => "AddEvent",
        }
    }

    fn as_path(&self) -> Vec<&str> {
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

impl GetPaths<{ Self::LEN }> for EngineGlobals {
    fn as_crate(&self) -> Crates {
        match self {
            EngineGlobals::CFoo | EngineGlobals::EFoo => Crates::Main,
            _ => Crates::Engine,
        }
    }

    fn as_ident(&self) -> &str {
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

    fn as_path(&self) -> Vec<&str> {
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
pub enum EngineIdents {
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
    // Use statements
    SDL2,
    SDL2Image,
}

impl GetPaths<{ Self::LEN }> for EngineIdents {
    fn as_crate(&self) -> Crates {
        Crates::Engine
    }

    fn as_ident(&self) -> &str {
        match self {
            EngineIdents::Container => "Container",
            EngineIdents::Label => "Label",
            EngineIdents::AndLabels => "AndLabels",
            EngineIdents::OrLabels => "OrLabels",
            EngineIdents::NandLabels => "NandLabels",
            EngineIdents::NorLabels => "NorLabels",
            EngineIdents::Singleton => "Singleton",
            EngineIdents::Intersect => "intersect",
            EngineIdents::IntersectMut => "intersect_mut",
            EngineIdents::IntersectKeys => "intersect_keys",
            EngineIdents::GetKeys => "get_keys",
            EngineIdents::CoreUpdate => "Update",
            EngineIdents::CoreEvents => "Events",
            EngineIdents::CorePreRender => "PreRender",
            EngineIdents::CoreRender => "Render",
            EngineIdents::Entity => "Entity",
            EngineIdents::EntitySet => "EntitySet",
            EngineIdents::EntityMap => "EntityMap",
            EngineIdents::SDL2 => "sdl2",
            EngineIdents::SDL2Image => "sdl2_image",
        }
    }

    fn as_path(&self) -> Vec<&str> {
        match self {
            EngineIdents::Container
            | EngineIdents::Label
            | EngineIdents::AndLabels
            | EngineIdents::OrLabels
            | EngineIdents::NandLabels
            | EngineIdents::NorLabels
            | EngineIdents::Singleton => vec![ECS, "components"],
            EngineIdents::Intersect
            | EngineIdents::IntersectMut
            | EngineIdents::IntersectKeys
            | EngineIdents::GetKeys => vec!["intersect"],
            EngineIdents::CoreUpdate
            | EngineIdents::CoreEvents
            | EngineIdents::CorePreRender
            | EngineIdents::CoreRender => {
                vec![ECS, "events", "core"]
            }
            EngineIdents::Entity | EngineIdents::EntitySet | EngineIdents::EntityMap => {
                vec![ECS, "entities"]
            }
            EngineIdents::SDL2 | EngineIdents::SDL2Image => vec![],
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

impl GetPaths<{ Self::LEN }> for NamespaceTraits {
    fn as_crate(&self) -> Crates {
        Crates::Engine
    }

    fn as_ident(&self) -> &str {
        match self {
            NamespaceTraits::AddComponent => EngineTraits::AddComponent.as_ident(),
            NamespaceTraits::AddEvent => EngineTraits::AddEvent.as_ident(),
        }
    }

    fn as_path(&self) -> Vec<&str> {
        vec![NAMESPACE]
    }
}

// All named crate indices
#[shared::macros::expand_enum]
pub enum Crates {
    Main,
    Engine,
    Macros,
}

#[derive(Clone, Debug)]
pub struct Paths {
    pub macros: [Path; MacroPaths::LEN],
    pub traits: [Path; EngineTraits::LEN],
    pub globals: [Path; EngineGlobals::LEN],
    pub idents: [Path; EngineIdents::LEN],
    pub cr_idxs: [usize; Crates::LEN],
}

impl Paths {
    pub fn new(engine_cr_idx: usize, macros_cr_idx: usize) -> Self {
        let mut cr_idxs = [0; Crates::LEN];
        cr_idxs[Crates::Main as usize] = 0;
        cr_idxs[Crates::Engine as usize] = engine_cr_idx;
        cr_idxs[Crates::Macros as usize] = macros_cr_idx;

        Self {
            macros: MacroPaths::crate_paths(&cr_idxs),
            traits: EngineTraits::crate_paths(&cr_idxs),
            globals: EngineGlobals::crate_paths(&cr_idxs),
            idents: EngineIdents::crate_paths(&cr_idxs),
            cr_idxs,
        }
    }

    pub fn get_cr_idx(&self, i: Crates) -> usize {
        self.cr_idxs[i as usize]
    }

    pub fn get_macro(&self, i: MacroPaths) -> &Path {
        &self.macros[i as usize]
    }

    pub fn get_trait(&self, i: EngineTraits) -> &Path {
        &self.traits[i as usize]
    }

    pub fn get_global(&self, i: EngineGlobals) -> &Path {
        &self.globals[i as usize]
    }

    pub fn get_ident(&self, i: EngineIdents) -> &Path {
        &self.idents[i as usize]
    }
}
