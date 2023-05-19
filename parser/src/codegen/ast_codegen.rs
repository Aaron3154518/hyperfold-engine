use std::{fs, path::PathBuf};

use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use shared::util::Call;
use shared::util::Increment;
use shared::util::JoinMapInto;
use shared::util::Unzip3;
use shared::util::Unzip7;
use shared::util::{Catch, JoinMap};

use crate::codegen;
use crate::codegen::idents::Idents;
use crate::codegen::util::vec_to_path;
use crate::resolve::ast_items::Dependency;
use crate::resolve::ast_paths::EngineGlobals;
use crate::resolve::ast_paths::EngineIdents;
use crate::resolve::ast_paths::EngineTraits;
use crate::resolve::ast_paths::ExpandEnum;
use crate::resolve::ast_paths::GetPaths;
use crate::resolve::ast_resolve::Path;
// use crate::validate::ast_item_list::ItemList;
use crate::validate::constants::component_var;
use crate::validate::constants::event_var;
use crate::validate::constants::event_variant;
use crate::validate::constants::global_var;
use crate::{
    resolve::{ast_items::ItemsCrate, ast_paths::Paths},
    validate::constants::NAMESPACE,
};

use super::util::string_to_type;

pub const INDEX: &str = "Index.txt";

pub fn codegen(paths: &Paths, crates: &mut Vec<ItemsCrate>) {
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("No out directory specified"));

    // Sort by crate index
    crates.sort_by_key(|cr| cr.cr_idx);

    // Create index file
    fs::write(
        out.join(INDEX),
        crates
            .map_vec(|cr| cr.dir.to_string_lossy().to_string())
            .join("\n"),
    )
    .catch(format!("Could not write to index file: {INDEX}"));

    // Write codegen
    for cr in crates.iter() {
        fs::write(
            out.join(format!("{}.rs", cr.cr_idx)),
            cr.codegen(paths, crates).to_string(),
        )
        .catch(format!("Could not write to {}.rs", cr.cr_idx));
    }
}

// Get dependencies in post order (children first)
pub trait GetPostOrder {
    fn get_post_order(&self) -> Vec<usize> {
        let mut v = Vec::new();
        self.get_post_order_impl(&mut v, 0);
        v
    }

    fn get_post_order_impl(&self, list: &mut Vec<usize>, cr_idx: usize);
}

impl GetPostOrder for Vec<ItemsCrate> {
    fn get_post_order_impl(&self, list: &mut Vec<usize>, cr_idx: usize) {
        for d in self[cr_idx].dependencies.iter() {
            if !list.contains(&d.cr_idx) {
                self.get_post_order_impl(list, d.cr_idx);
            }
        }
        list.push(cr_idx);
    }
}

impl ItemsCrate {
    fn get_engine_trait_paths(
        &self,
        paths: &Paths,
        crates: &Vec<ItemsCrate>,
    ) -> [syn::Type; EngineTraits::LEN] {
        paths
            .traits
            .each_ref()
            .map(|p| string_to_type(p.path_from(self.cr_idx, crates).join("::"), false, false))
    }

    fn get_crate_paths(&self, crates: &Vec<ItemsCrate>) -> Vec<syn::Path> {
        crates.map_vec(|cr| {
            vec_to_path(
                Path {
                    cr_idx: cr.cr_idx,
                    path: vec![if cr.cr_idx == 0 {
                        "crate".to_string()
                    } else {
                        cr.cr_name.to_string()
                    }],
                }
                .path_from(self.cr_idx, crates),
            )
        })
    }

    fn get_engine_globals(
        &self,
        paths: &Paths,
        crates: &Vec<ItemsCrate>,
    ) -> [(usize, usize); EngineGlobals::LEN] {
        paths.globals.each_ref().map(|gl| {
            (
                gl.cr_idx,
                crates[gl.cr_idx]
                    .globals
                    .iter()
                    .position(|g| &g.path == gl)
                    .catch(format!(
                        "Could not find global: {} in crate {}",
                        gl.path.join("::"),
                        gl.cr_idx
                    )),
            )
        })
    }

    // Must update generated mod as well
    fn codegen_dep(&self, paths: &Paths, crates: &Vec<ItemsCrate>) -> TokenStream {
        let engine_paths = self.get_engine_trait_paths(paths, crates);
        let ns = format_ident!("{}", NAMESPACE);
        let dep_aliases = self
            .dependencies
            .iter()
            .map(|d| format_ident!("{}", d.cr_alias))
            .collect::<Vec<_>>();

        // Aggregate AddComponent traits and dependencies
        let add_comp = Idents::AddComponent.to_ident();
        let add_comp_tr = &engine_paths[EngineTraits::AddComponent as usize];
        let mut comp_trs = self.components.map_vec(|c| {
            let ty = vec_to_path(c.path.path.to_vec());
            quote!(#add_comp_tr<#ty>)
        });
        comp_trs.append(&mut dep_aliases.map_vec(|da| quote!(#da::#ns::#add_comp)));
        let comp_code = match comp_trs.split_first() {
            Some((first, tail)) => {
                quote!(
                    pub trait #add_comp: #first #(+#tail)* {}
                )
            }
            None => quote!(pub trait #add_comp {}),
        };

        // Aggregate AddEvent traits and dependencies
        let add_event = Idents::AddEvent.to_ident();
        let add_event_tr = &engine_paths[EngineTraits::AddEvent as usize];
        let mut event_trs = self.events.map_vec(|e| {
            let ty = vec_to_path(e.path.path.to_vec());
            quote!(#add_event_tr<#ty>)
        });
        event_trs.append(&mut dep_aliases.map_vec(|da| quote!(#da::#ns::#add_event)));
        let event_code = match event_trs.split_first() {
            Some((first, tail)) => {
                quote!(
                    pub trait #add_event: #first #(+#tail)* {}
                )
            }
            None => quote!(pub trait #add_event {}),
        };

        quote!(
            #(pub use #dep_aliases;)*
            #comp_code
            #event_code
        )
    }

    fn codegen_entry(&self, paths: &Paths, crates: &Vec<ItemsCrate>) -> TokenStream {
        // Use statements and traits
        let deps_code = self.codegen_dep(paths, crates);

        let deps_lrn = crates.get_post_order();
        // Paths from entry crate to all other crates
        let crate_paths = self.get_crate_paths(crates);
        let crate_paths_post = deps_lrn.map_vec(|i| &crate_paths[*i]);
        let engine_cr_path = &crate_paths[paths.engine_cr_idx];

        // Get component types by crate for system codegen
        // Get full list of components, global, evnets, and systems
        let (c_tys, c_vars, g_tys, g_vars, e_tys, e_vars, e_varis) = crates
            .iter()
            .zip(crate_paths.iter())
            .collect::<Vec<_>>()
            .map_vec(|&(cr, cr_path)| {
                (
                    cr.components.iter().enumerate().unzip_vec(|(i, c)| {
                        let path = vec_to_path(c.path.path[1..].to_vec());
                        (
                            quote!(#cr_path::#path),
                            format_ident!("{}", component_var(self.cr_idx, i)),
                        )
                    }),
                    cr.globals.iter().enumerate().unzip_vec(|(i, g)| {
                        let path = vec_to_path(g.path.path[1..].to_vec());
                        (
                            quote!(#cr_path::#path),
                            format_ident!("{}", global_var(self.cr_idx, i)),
                        )
                    }),
                    cr.events
                        .iter()
                        .enumerate()
                        .map_vec(|(i, e)| {
                            let ty = vec_to_path(e.path.path[1..].to_vec());
                            (
                                quote!(#cr_path::#ty),
                                format_ident!("{}", event_var(cr.cr_idx, i)),
                                format_ident!("{}", event_variant(cr.cr_idx, i)),
                            )
                        })
                        .into_iter()
                        .unzip3_vec(),
                )
                    .call_into(
                        |((c_tys, c_vars), (g_tys, g_vars), (e_tys, e_vars, e_varis))| {
                            (c_tys, c_vars, g_tys, g_vars, e_tys, e_vars, e_varis)
                        },
                    )
            })
            .into_iter()
            .unzip7_vec();

        // Global manager
        let gfoo_ident = Idents::GFoo.to_ident();
        let gfoo_def = quote!(
            pub struct #gfoo_ident {
                #(#(#g_vars: #g_tys),*),*
            }

            impl #gfoo_ident {
                pub fn new() -> Self {
                    Self {
                        #(#(#g_vars: #g_tys::new()),*,)*
                    }
                }
            }
        );

        // Namespace identifier
        let ns = Idents::Namespace.to_ident();
        // Paths engine traits
        let engine_trait_paths = self.get_engine_trait_paths(paths, crates);

        let engine_globals = self.get_engine_globals(paths, crates);
        // Paths to engine globals
        let entity = EngineIdents::Entity
            .to_path()
            .call(|i| quote!(#engine_cr_path::#i));
        let entity_trash = EngineGlobals::EntityTrash
            .to_path()
            .call(|i| quote!(#engine_cr_path::#i));

        // Component manager
        let add_comp = Idents::AddComponent.to_ident();
        let add_comp_tr = &engine_trait_paths[EngineTraits::AddComponent as usize];
        let cfoo_ident = Idents::CFoo.to_ident();
        let cfoo_def = quote!(
            pub struct #cfoo_ident {
                eids: std::collections::HashSet<#entity>,
                #(#(#c_vars: std::collections::HashMap<#entity, #c_tys>),*,)*
            }

            impl #cfoo_ident {
                pub fn new() -> Self {
                    Self {
                        eids: std::collections::HashSet::new(),
                        #(#(#c_vars: std::collections::HashMap::new()),*,)*
                    }
                }

                pub fn append(&mut self, cm: &mut Self) {
                    self.eids.extend(cm.eids.drain());
                    #(#(self.#c_vars.extend(cm.#c_vars.drain());)*)*
                }

                pub fn remove(&mut self, tr: &mut #entity_trash) {
                    for eid in tr.0.drain(..) {
                        self.eids.remove(&eid);
                        #(#(self.#c_vars.remove(&eid);)*)*
                    }
                }
            }
        );
        let cfoo_traits = quote!(
            #(#(
                impl #add_comp_tr<#c_tys> for #cfoo_ident {
                    fn add_component(&mut self, e: #entity, t: #c_tys) {
                        self.eids.insert(e);
                        self.#c_vars.insert(e, t);
                    }
                }
            )*)*
            #(
                impl #crate_paths_post::#ns::#add_comp for #cfoo_ident {}
            )*
        );

        // Event manager
        let add_event = Idents::AddEvent.to_ident();
        let add_event_tr = &engine_trait_paths[EngineTraits::AddEvent as usize];
        let e_ident = Idents::E.to_ident();
        let e_len_ident = Idents::ELen.to_ident();
        let e_len = e_vars.iter().fold(0, |s, v| s + v.len());
        let e_def = quote!(
            #[derive(Hash, Clone, Copy, Eq, PartialEq, Debug)]
            pub enum #e_ident {
                #(#(#e_varis),*),*
            }
            pub const #e_len_ident: usize = #e_len;
        );
        let efoo_ident = Idents::EFoo.to_ident();
        let efoo_def = quote!(
            pub struct #efoo_ident {
                #(#(#e_vars: Vec<#e_tys>),*),*,
                events: std::collections::VecDeque<(#e_ident, usize)>
            }

            impl #efoo_ident {
                pub fn new() -> Self {
                    Self {
                        #(#(#e_vars: Vec::new()),*),*,
                        events: std::collections::VecDeque::new()
                    }
                }

                pub fn has_events(&self) -> bool {
                    !self.events.is_empty()
                }

                fn add_event(&mut self, e: #e_ident) {
                    self.events.push_back((e, 0));
                }

                pub fn get_events(&mut self) -> std::collections::VecDeque<(#e_ident, usize)> {
                    std::mem::replace(&mut self.events, std::collections::VecDeque::new())
                }

                pub fn append(&mut self, other: &mut Self) {
                    #(#(
                        other.#e_vars.reverse();
                        self.#e_vars.append(&mut other.#e_vars);
                    )*)*
                }

                pub fn pop(&mut self, e: #e_ident) {
                    match e {
                        #(#(
                            #e_ident::#e_varis => {
                                self.#e_vars.pop();
                            }
                        )*)*
                    }
                }
            }
        );
        let efoo_traits = quote!(
            #(#(
                impl #add_event_tr<#e_tys> for #efoo_ident {
                    fn new_event(&mut self, t: #e_tys) {
                        self.#e_vars.push(t);
                        self.add_event(#e_ident::#e_varis);
                    }

                    fn get_event<'a>(&'a self) -> Option<&'a #e_tys> {
                        self.#e_vars.last()
                    }
                }
            )*)*
            #(
                impl #crate_paths_post::#ns::#add_event for #efoo_ident {}
            )*
        );

        // Variables for engine globals
        let engine_globals =
            engine_globals.map(|(cr_idx, g_i)| format_ident!("{}", global_var(cr_idx, g_i)));
        let g_event = &engine_globals[EngineGlobals::Event as usize];
        let g_render_system = &engine_globals[EngineGlobals::RenderSystem as usize];
        let g_entity_trash = &engine_globals[EngineGlobals::EntityTrash as usize];
        let g_screen = &engine_globals[EngineGlobals::Screen as usize];
        let g_camera = &engine_globals[EngineGlobals::Camera as usize];
        let g_efoo = &engine_globals[EngineGlobals::EFoo as usize];
        let g_cfoo = &engine_globals[EngineGlobals::CFoo as usize];

        // Systems
        let (init_systems, systems) = crates.iter().zip(crate_paths.iter()).fold(
            (Vec::new(), Vec::new()),
            |(mut s_init, mut sys), (cr, cr_path)| {
                for s in cr.systems.iter() {
                    let s = codegen::system::System::from(s, paths, crates);
                    if s.is_init { &mut s_init } else { &mut sys }
                        .push(s.to_quote(&engine_cr_path, &c_tys))
                }
                (s_init, sys)
            },
        );

        let [cfoo, gfoo, efoo] =
            [Idents::GenCFoo, Idents::GenGFoo, Idents::GenEFoo].map(|i| i.to_ident());
        let [core_update, core_events, core_render] = [
            EngineIdents::CoreUpdate,
            EngineIdents::CoreEvents,
            EngineIdents::CoreRender,
        ]
        .map(|i| vec_to_path(i.path_stem()));

        // Sdl
        let sdl2 = EngineIdents::SDL2.to_path();

        // Systems manager
        let sfoo_ident = Idents::SFoo.to_ident();
        let sfoo_struct = quote!(
            pub struct #sfoo_ident {
                #cfoo: #cfoo_ident,
                #gfoo: #gfoo_ident,
                #efoo: #efoo_ident,
                stack: Vec<std::collections::VecDeque<(#e_ident, usize)>>,
                services: [Vec<Box<dyn Fn(&mut #cfoo_ident, &mut #gfoo_ident, &mut #efoo_ident)>>; #e_len_ident]
            }
        );
        let sfoo_new = quote!(
            pub fn new() -> Self {
                let mut s = Self {
                    #cfoo: #cfoo_ident::new(),
                    #gfoo: #gfoo_ident::new(),
                    #efoo: #efoo_ident::new(),
                    stack: Vec::new(),
                    services: std::array::from_fn(|_| Vec::new())
                };
                s.init();
                s
            }
        );
        let sfoo_init = quote!(
            // Init
            fn init(&mut self) {
                #(#init_systems)*
                self.post_tick();
                self.add_systems();
            }

            fn add_system(&mut self, e: #e_ident, f: Box<dyn Fn(&mut #cfoo_ident, &mut #gfoo_ident, &mut #efoo_ident)>) {
                self.services[e as usize].push(f);
            }

            fn add_systems(&mut self) {
                #(#systems)*
            }
        );
        let sfoo_run = quote!(
            pub fn run(&mut self) {
                static FPS: u32 = 60;
                static FRAME_TIME: u32 = 1000 / FPS;

                let mut t = unsafe { #engine_cr_path::#sdl2::SDL_GetTicks() };
                let mut dt;
                let mut tsum: u64 = 0;
                let mut tcnt: u64 = 0;
                while !self.#gfoo.#g_event.quit {
                    dt = unsafe { #engine_cr_path::#sdl2::SDL_GetTicks() } - t;
                    t += dt;

                    self.tick(dt);

                    dt = unsafe { #engine_cr_path::#sdl2::SDL_GetTicks() } - t;
                    tsum += dt as u64;
                    tcnt += 1;
                    if dt < FRAME_TIME {
                        unsafe { #engine_cr_path::#sdl2::SDL_Delay(FRAME_TIME - dt) };
                    }
                }

                println!("Average Frame Time: {}ms", tsum as f64 / tcnt as f64);
            }
        );
        let sfoo_tick = quote!(
            fn tick(&mut self, ts: u32) {
                // Update #efoo
                self.#gfoo.#g_event.update(ts, &self.#gfoo.#g_camera.0, &self.#gfoo.#g_screen.0);
                // Clear the screen
                self.#gfoo.#g_render_system.r.clear();
                // Add initial #efoo
                self.add_events(self.init_events(ts));
                while !self.stack.is_empty() {
                    // Get element from next queue
                    if let Some((e, i, n)) = self
                        .stack
                        // Get last queue
                        .last_mut()
                        // Get next #efoo
                        .and_then(|queue| queue.front_mut())
                        // Check if the system exists
                        .and_then(|(e, i)| {
                            // Increment the event idx and return the old values
                            let v_s = &self.services[*e as usize];
                            v_s.get(*i).map(|_| {
                                let vals = (e.clone(), i.clone(), v_s.len());
                                *i += 1;
                                vals
                            })
                        })
                    {
                        // This is the last system for this event
                        if i + 1 >= n {
                            self.pop();
                        }
                        // Add a new queue for new #efoo
                        self.#gfoo.#g_efoo = #efoo_ident::new();
                        // Run the system
                        if let Some(s) = self.services[e as usize].get(i) {
                            (s)(&mut self.#cfoo, &mut self.#gfoo, &mut self.#efoo);
                        }
                        // If this is the last system, remove the event
                        if i + 1 >= n {
                            self.#efoo.pop(e);
                        }
                        // Add new #efoo
                        let #efoo = std::mem::replace(&mut self.#gfoo.#g_efoo, #efoo_ident::new());
                        self.add_events(#efoo);
                    } else {
                        // We're done with this event
                        self.pop();
                    }
                }
                // Display the screen
                self.#gfoo.#g_render_system.r.present();

                self.post_tick();
            }

            fn post_tick(&mut self) {
                // Remove marked entities
                self.#cfoo.remove(&mut self.#gfoo.#g_entity_trash);
                // Add new entities
                self.#cfoo.append(&mut self.#gfoo.#g_cfoo);
            }
        );
        let sfoo_events = quote!(
            fn init_events(&self, ts: u32) -> #efoo_ident {
                let mut #efoo = #efoo_ident::new();
                #add_event_tr::new_event(&mut #efoo, #engine_cr_path::#core_events);
                #add_event_tr::new_event(&mut #efoo, #engine_cr_path::#core_update(ts));
                #add_event_tr::new_event(&mut #efoo, #engine_cr_path::#core_render);
                #efoo
            }

            fn add_events(&mut self, mut em: #efoo_ident) {
                if em.has_events() {
                    self.#efoo.append(&mut em);
                    self.stack.push(em.get_events());
                }
            }

            fn pop(&mut self) {
                // Remove top element and empty queue
                if self.stack.last_mut().is_some_and(|queue| {
                    queue.pop_front();
                    queue.is_empty()
                }) {
                    self.stack.pop();
                }
            }
        );
        let sfoo_def = quote!(
            #sfoo_struct

            impl #sfoo_ident {
                #sfoo_new

                #sfoo_init

                #sfoo_run

                #sfoo_tick

                #sfoo_events
            }
        );

        quote!(
            #deps_code

            #gfoo_def

            #cfoo_def
            #cfoo_traits

            #e_def
            #efoo_def
            #efoo_traits

            #sfoo_def
        )
    }

    pub fn codegen(&self, paths: &Paths, crates: &Vec<ItemsCrate>) -> TokenStream {
        let ns: proc_macro2::Ident = format_ident!("{}", NAMESPACE);
        let code = match self.cr_idx {
            0 => self.codegen_entry(paths, crates),
            _ => self.codegen_dep(paths, crates),
        };
        quote!(
            pub mod #ns {
                #code
            }
        )
    }
}