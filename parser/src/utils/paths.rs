use once_cell::sync::Lazy;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use shared::{
    macros::{expand_enum, ExpandEnum},
    match_ok,
    msg_result::Zip7Msgs,
    traits::{Call, CollectVec, PushInto},
};

use crate::{
    codegen::Crates,
    parse::{resolve_path_from_crate, DiscardSymbol, GlobalSymbol, ItemPath, MatchSymbol},
    utils::{constants::NAMESPACE, idents::global_var, syn::vec_to_path, Msg, MsgResult},
};

// All named crate indices
#[expand_enum]
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

impl std::fmt::Display for CratePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:#?}{}::{}",
            self.cr,
            self.path.map_vec(|s| format!("::{s}")).join(""),
            self.ident
        )
    }
}

macro_rules! paths {
    (@str_var { $var: ident }) => { $var };

    (@str_var $str: ident) => { stringify!($str) };

    (@const $const: ident, $ty: ident) => {
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
        state => state,
    },
    Engine { components => components }
});

// Paths to base trait definitions use in codegen
paths!(ENGINE_TRAITS = EngineTraits {
    Engine::ecs::components {
        add_component => AddComponent,
    },
    Engine::ecs::events {
        add_event => AddEvent,
        set_state => SetState,
    },
    Main::{NAMESPACE} {
        components => Components,
        events => Events,
    }
});

pub struct TraitPath<'a> {
    pub main_trait: &'a CratePath,
    pub global: &'a CratePath,
}

pub const TRAITS: Lazy<[TraitPath; 2]> = Lazy::new(|| {
    [
        TraitPath {
            main_trait: &ENGINE_TRAITS.components,
            global: &ENGINE_GLOBALS.c_foo,
        },
        TraitPath {
            main_trait: &ENGINE_TRAITS.events,
            global: &ENGINE_GLOBALS.e_foo,
        },
    ]
});

// Paths to engine globals needed by codegen
macro_rules! engine_globals {
    ($const: ident = $ty: ident {
        $($cr: ident $(::$path: tt)* {
            $($var: ident => $ident: tt),* $(,)?
        }),* $(,)?
    }, $ty_res: ident, $zip_tr: ident) => {
        paths!($const = $ty {
            $($cr $(::$path)* {
                $($var => $ident),*
            }),*
        });

        pub struct $ty_res {
            $($(pub $var: syn::Ident),*),*
        }

        impl $ty {
            pub fn get_global_vars(&self, crates: &Crates, cr_idx: usize) -> MsgResult<$ty_res> {
                let cr = match crates.get(cr_idx) {
                    Some(cr) => cr,
                    None => return Err(vec![Msg::String(format!("Invalid crate index: {cr_idx}"))]),
                };
                let get_global = |cr_path| {
                    crates
                        .get_path(cr_idx, cr_path)
                        .and_then(|path| {
                            resolve_path_from_crate(path, cr, crates.get_crates())
                                .expect_global()
                                .discard_symbol()
                        })
                        .map(|g_sym| global_var(g_sym.idx))
                };
                $($(let $var = get_global(&self.$var);)*)*
                match_ok!($zip_tr $($(,$var)*)*, {
                    $ty_res { $($($var),*),* }
                })
            }
        }
    };
}

engine_globals!(ENGINE_GLOBALS = EngineGlobals {
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
}, EngineGlobalPaths, Zip7Msgs);

// Paths to engine items needed by parsing
paths!(ENGINE_PATHS = EnginePaths {
    // Components
    Engine::ecs::components { singleton => Singleton, },
    // Functions
    Engine::intersect {
        filter => filter,
        intersect => intersect,
        intersect_opt => intersect_opt
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
    Engine {
        // Use statements
        sdl2 => sdl2,
        sdl2_image => sdl2_image,
        // Manager
        manager_trait => ManagerTrait
    },
});

// Use statements for the namespace
pub const NAMESPACE_USE_STMTS: Lazy<[&CratePath; 1]> = Lazy::new(|| [&ENGINE_PATHS.entity]);

pub const MAIN_USE_STMTS: Lazy<[&CratePath; 2]> =
    Lazy::new(|| [&ENGINE_TRAITS.add_event, &ENGINE_TRAITS.set_state]);
