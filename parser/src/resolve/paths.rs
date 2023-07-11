use crate::{
    codegen2::{util::vec_to_path, Crates},
    resolve::constants::NAMESPACE,
    util::end,
};
use once_cell::sync::Lazy;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use shared::util::{Call, JoinMap, PushInto};

use super::{path::ItemPath, util::MsgsResult};

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

pub struct CratePath {
    pub cr: Crate,
    pub path: Vec<&'static str>,
    pub ident: &'static str,
}

impl CratePath {
    pub fn get_ident(&self) -> String {
        self.ident.to_string()
    }

    pub fn full_path(&self) -> Vec<String> {
        [
            vec!["crate".to_string()],
            self.path
                .map_vec(|s| s.to_string())
                .push_into(self.ident.to_string()),
        ]
        .concat()
    }
}

macro_rules! paths {
    (@str_var { $var: ident }) => { $var };

    (@str_var $str: ident) => { stringify!($str) };

    (@const $const: ident, $ty: ident) => {
        pub static $const: Lazy<$ty> = Lazy::new(|| $ty::new());
    };

    ($const: ident = $ty: ident {
        $($var: ident => <$cr: ident $(::$path: tt)*> :: $ident: tt),* $(,)?
    }) => {
        pub struct $ty {
            $(pub $var: CratePath),*
        }

        impl $ty {
            pub fn new() -> Self {
                Self {
                    $($var: CratePath {
                        cr: Crate::$cr,
                        path: vec![$(paths!(@str_var $path)),*],
                        ident: paths!(@str_var $ident),
                    }),*
                }
            }

            pub fn paths(&self) -> Vec<&CratePath> {
                vec![$(&self.$var),*]
            }
        }

        pub static $const: Lazy<$ty> = Lazy::new(|| $ty::new());
    };

    ($const: ident = $ty: ident {
        $($cr: ident $(::$path: tt)* {
            $($var: ident => $ident: tt),* $(,)?
        }),* $(,)?
    }) => {
        pub struct $ty {
            $($(pub $var: CratePath),*),*
        }

        impl $ty {
            pub fn new() -> Self {
                let mut s = Self {
                    $($($var: CratePath {
                        cr: Crate::$cr,
                        path: vec![],
                        ident: paths!(@str_var $ident),
                    }),*),*
                };
                $([$(s.$var.path),*] = std::array::from_fn(|_| vec![$(paths!(@str_var $path)),*]);)*
                s
            }

            pub fn paths(&self) -> Vec<&CratePath> {
                vec![$($(&self.$var),*),*]
            }
        }

        pub static $const: Lazy<$ty> = Lazy::new(|| $ty::new());
    };
}

// Paths to marker macros
paths!(MACRO_PATHS = MacroPaths {
    Macros {
        component => component,
        global => global,
        event => event,
        system => system,
    },
    Engine { components => components }
});

// Paths to base trait definitions use in codegen
paths!(ENGINE_TRAITS = EngineTraits {
    Engine::ecs::components {
        add_component => AddComponent,
        add_event => AddEvent,
    },
    Main::{NAMESPACE} {
        main_add_component => AddComponent,
        main_add_event => AddEvent,
    }
});

pub struct TraitPath<'a> {
    pub main_trait: &'a CratePath,
    pub global_trait: &'a CratePath,
}

pub const TRAITS: Lazy<[TraitPath; 2]> = Lazy::new(|| {
    [
        TraitPath {
            main_trait: &ENGINE_TRAITS.main_add_component,
            global_trait: &ENGINE_TRAITS.add_component,
        },
        TraitPath {
            main_trait: &ENGINE_TRAITS.main_add_event,
            global_trait: &ENGINE_TRAITS.add_event,
        },
    ]
});

// Paths to engine globals needed by codegen
paths!(ENGINE_GLOBALS = EngineGlobals {
    Main::{NAMESPACE} {
        c_foo => CFoo,
        e_foo => EFoo,
    },
    Engine::ecs::entities { entity_trash => EntityTrash },
    Engine::utils::event { event => Event },
    Engine::framework::render_system {
        renderer => Renderer,
        camera => Camera,
        screen => Screen,
    }
});

// Paths to engine items needed by parsing
paths!(ENGINE_PATHS = EnginePaths {
    // Components
    Engine::ecs::components { singleton => Singleton, },
    // Functions
    Engine::intersect {
        filter => Filter,
        intersect => Intersect
    },
    // Events
    Engine::ecs::events::core {
        core_update => Update,
        core_events => Events,
        core_pre_render => PreRender,
        core_render => Render
    },
    // Entities
    Engine::ecs::entities {
        entity => Entity,
        entity_set => EntitySet,
        entity_map => EntityMap
    },
    // Systems
    Engine::ecs::systems {
        entities => Entities,
    },
    // Use statements
    Engine {
        sdl2 => sdl2,
        sdl2_image => sdl2_image
    },
});
