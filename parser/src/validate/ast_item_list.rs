use crate::{
    resolve::{
        ast_items::{Component, Event, Global, ItemsCrate, Trait},
        ast_paths::{EngineTraits, ExpandEnum, GetPaths, NamespaceTraits, Paths},
        ast_resolve::Path,
    },
    util::{Catch, JoinMap, JoinMapInto},
};

// This struct is used to modify the lists of items
// Traits: hardcodes traits
#[derive(Debug)]
pub struct ItemList<'a> {
    pub components: Vec<Vec<&'a Component>>,
    pub globals: Vec<Vec<&'a Global>>,
    pub events: Vec<Vec<&'a Event>>,
    pub traits: Vec<Trait>,
}

impl<'a> ItemList<'a> {
    pub fn from(crates: &'a Vec<ItemsCrate>, paths: &Paths) -> Self {
        let (mut components, mut globals, mut events) = (Vec::new(), Vec::new(), Vec::new());
        for cr in crates.iter() {
            components.push(cr.components.iter().collect());
            globals.push(cr.globals.iter().collect());
            events.push(cr.events.iter().collect());
        }

        Self {
            components,
            globals,
            events,
            traits: NamespaceTraits::VARIANTS.iter().map_vec(|tr| {
                let gl = &paths.globals[tr.get_global() as usize];
                Trait {
                    path: Path {
                        cr_idx: 0,
                        path: tr.full_path(),
                    },
                    cr_idx: 0,
                    g_idx: crates[0]
                        .globals
                        .iter()
                        .position(|g| g.path.path == gl.path)
                        .catch(format!(
                            "Could not find trait global: {}",
                            gl.path.join("::")
                        )),
                }
            }),
        }
    }
}
