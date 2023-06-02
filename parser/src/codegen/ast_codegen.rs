use std::{array, env::temp_dir, fs, path::PathBuf};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use shared::{
    let_mut_vecs,
    util::{Call, Catch, FlattenMap, Increment, JoinMap, JoinMapInto, Unzip3, Unzip8},
};

use crate::{
    codegen::{self, idents::Idents, structs::Component, util::vec_to_path},
    resolve::{
        ast_items::{Dependency, ItemsCrate},
        ast_paths::{EngineGlobals, EngineIdents, EngineTraits, ExpandEnum, GetPaths, Paths},
        ast_resolve::Path,
    },
    validate::constants::{component_var, event_var, event_variant, global_var, NAMESPACE},
};

use super::util::string_to_type;

pub const INDEX: &str = "hyperfold_engine_index.txt";
pub const INDEX_SEP: &str = "\t";

pub fn codegen(paths: &Paths, crates: &mut Vec<ItemsCrate>) {
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("No out directory specified"));

    // Sort by crate index
    crates.sort_by_key(|cr| cr.cr_idx);

    // Write codegen
    for cr in crates.iter() {}

    // Create index file
    fs::write(
        temp_dir().join(INDEX),
        crates
            .map_vec(|cr| {
                // Write to file
                let file = out.join(format!("{}.rs", cr.cr_idx));
                fs::write(file.to_owned(), cr.codegen(paths, crates).to_string())
                    .catch(format!("Could not write to: {}", file.display()));
                format!(
                    "{}{}{}",
                    cr.dir.to_string_lossy().to_string(),
                    INDEX_SEP,
                    file.display()
                )
            })
            .join("\n"),
    )
    .catch(format!("Could not write to index file: {INDEX}"));
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

        let singleton = EngineIdents::Singleton.construct_path(&engine_cr_path);
        let entity_set = EngineIdents::EntitySet.construct_path(&engine_cr_path);
        let entity_map = EngineIdents::EntityMap.construct_path(&engine_cr_path);

        // Get component types by crate for system codegen
        let mut comps = Vec::new();
        let_mut_vecs!(cfoo_tys, cfoo_news, cfoo_adds, cfoo_appends, cfoo_removes);
        let_mut_vecs!(g_tys, g_vars, e_tys, e_vars, e_varis);
        for (cr, cr_path) in crates.iter().zip(crate_paths.iter()) {
            comps.push(cr.components.iter().enumerate().map_vec(|(i, c)| {
                let path = vec_to_path(c.path.path[1..].to_vec());
                let ty = quote!(#cr_path::#path);
                let var = format_ident!("{}", component_var(cr.cr_idx, i));

                if c.args.is_singleton {
                    cfoo_tys.push(quote!(Option<#singleton<#ty>>));
                    cfoo_news.push(quote!(None));
                    cfoo_adds.push(quote!(self.#var = Some(#singleton::new(e, t))));
                    cfoo_appends.push(quote!(
                        if cm.#var.is_some() {
                            if self.#var.is_some() {
                                panic!("Cannot set Singleton component more than once")
                            }
                            self.#var = cm.#var.take();
                        }
                    ));
                    cfoo_removes.push(quote!(
                        if self.#var.as_ref().is_some_and(|s| s.contains_key(&eid)) {
                            self.#var = None;
                        }
                    ));
                } else {
                    cfoo_tys.push(quote!(#entity_map<#ty>));
                    cfoo_news.push(quote!(#entity_map::new()));
                    cfoo_adds.push(quote!(self.#var.insert(e, t);));
                    cfoo_appends.push(quote!(self.#var.extend(cm.#var.drain());));
                    cfoo_removes.push(quote!(self.#var.remove(&eid);));
                }
                Component {
                    ty,
                    var,
                    args: c.args,
                }
            }));

            for (i, g) in cr.globals.iter().enumerate() {
                let path = vec_to_path(g.path.path[1..].to_vec());
                g_tys.push(quote!(#cr_path::#path));
                g_vars.push(format_ident!("{}", global_var(cr.cr_idx, i)));
            }
            for (i, e) in cr.events.iter().enumerate() {
                let ty = vec_to_path(e.path.path[1..].to_vec());
                e_tys.push(quote!(#cr_path::#ty));
                e_vars.push(format_ident!("{}", event_var(cr.cr_idx, i)));
                e_varis.push(format_ident!("{}", event_variant(cr.cr_idx, i)));
            }
        }
        let c_tys = comps.flatten_map_vec(|c| &c.ty);
        let c_vars = comps.flatten_map_vec(|c| &c.var);

        // Global manager
        let gfoo_ident = Idents::GFoo.to_ident();
        let gfoo_def = quote!(
            pub struct #gfoo_ident {
                #(#g_vars: #g_tys),*
            }

            impl #gfoo_ident {
                pub fn new() -> Self {
                    Self {
                        #(#g_vars: #g_tys::new()),*
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
        let entity_trash = EngineGlobals::EntityTrash.construct_path(&engine_cr_path);

        let entity = EngineIdents::Entity.construct_path(&engine_cr_path);

        // Component manager
        let add_comp = Idents::AddComponent.to_ident();
        let add_comp_tr = &engine_trait_paths[EngineTraits::AddComponent as usize];
        let cfoo_ident = Idents::CFoo.to_ident();
        let cfoo_def = quote!(
            pub struct #cfoo_ident {
                eids: #entity_set,
                #(#c_vars: #cfoo_tys),*
            }

            impl #cfoo_ident {
                pub fn new() -> Self {
                    Self {
                        eids: #entity_set::new(),
                        #(#c_vars: #cfoo_news),*
                    }
                }

                pub fn append(&mut self, cm: &mut Self) {
                    self.eids.extend(cm.eids.drain());
                    #(#cfoo_appends)*
                }

                pub fn remove(&mut self, tr: &mut #entity_trash) {
                    for eid in tr.0.drain(..) {
                        self.eids.remove(&eid);
                        #(#cfoo_removes)*
                    }
                }
            }
        );
        let cfoo_traits = quote!(
            #(
                impl #add_comp_tr<#c_tys> for #cfoo_ident {
                    fn add_component(&mut self, e: #entity, t: #c_tys) {
                        self.eids.insert(e);
                        #cfoo_adds
                    }
                }
            )*
            #(
                impl #crate_paths_post::#ns::#add_comp for #cfoo_ident {}
            )*
        );

        // Event manager
        let add_event = Idents::AddEvent.to_ident();
        let add_event_tr = &engine_trait_paths[EngineTraits::AddEvent as usize];
        let e_ident = Idents::E.to_ident();
        let e_len_ident = Idents::ELen.to_ident();
        let e_len = e_varis.len();
        let e_def = quote!(
            #[derive(Hash, Clone, Copy, Eq, PartialEq, Debug)]
            pub enum #e_ident {
                #(#e_varis),*
            }
            pub const #e_len_ident: usize = #e_len;
        );
        let efoo_ident = Idents::EFoo.to_ident();
        let efoo_def = quote!(
            pub struct #efoo_ident {
                #(#e_vars: Vec<#e_tys>),*,
                events: std::collections::VecDeque<(#e_ident, usize)>
            }

            impl #efoo_ident {
                pub fn new() -> Self {
                    Self {
                        #(#e_vars: Vec::new()),*,
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
                    #(
                        other.#e_vars.reverse();
                        self.#e_vars.append(&mut other.#e_vars);
                    )*
                }

                pub fn pop(&mut self, e: #e_ident) {
                    match e {
                        #(
                            #e_ident::#e_varis => {
                                self.#e_vars.pop();
                            }
                        )*
                    }
                }
            }
        );
        let efoo_traits = quote!(
            #(
                impl #add_event_tr<#e_tys> for #efoo_ident {
                    fn new_event(&mut self, t: #e_tys) {
                        self.#e_vars.push(t);
                        self.add_event(#e_ident::#e_varis);
                    }

                    fn get_event<'a>(&'a self) -> Option<&'a #e_tys> {
                        self.#e_vars.last()
                    }
                }
            )*
            #(
                impl #crate_paths_post::#ns::#add_event for #efoo_ident {}
            )*
        );

        // Variables for engine globals
        let engine_globals =
            engine_globals.map(|(cr_idx, g_i)| format_ident!("{}", global_var(cr_idx, g_i)));
        let g_event = &engine_globals[EngineGlobals::Event as usize];
        let g_renderer = &engine_globals[EngineGlobals::Renderer as usize];
        let g_entity_trash = &engine_globals[EngineGlobals::EntityTrash as usize];
        let g_screen = &engine_globals[EngineGlobals::Screen as usize];
        let g_camera = &engine_globals[EngineGlobals::Camera as usize];
        let g_efoo = &engine_globals[EngineGlobals::EFoo as usize];
        let g_cfoo = &engine_globals[EngineGlobals::CFoo as usize];

        // Systems
        let_mut_vecs!(systems, init_systems);
        for (cr, cr_path) in crates.iter().zip(crate_paths.iter()) {
            for s in cr.systems.iter() {
                let s = codegen::system::System::from(s, cr_path, paths, crates);
                if s.is_init {
                    &mut init_systems
                } else {
                    &mut systems
                }
                .push(s.to_quote(&engine_cr_path, &comps))
            }
        }

        let [cfoo, gfoo, efoo] =
            [Idents::GenCFoo, Idents::GenGFoo, Idents::GenEFoo].map(|i| i.to_ident());
        let [core_update, core_events, core_pre_render, core_render] = [
            EngineIdents::CoreUpdate,
            EngineIdents::CoreEvents,
            EngineIdents::CorePreRender,
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
                self.#gfoo.#g_renderer.clear();
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
                self.#gfoo.#g_renderer.present();

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
                #add_event_tr::new_event(&mut #efoo, #engine_cr_path::#core_pre_render);
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
            #[allow(unused_parens, unused_variables)]
            pub mod #ns {
                #code
            }
        )
    }
}
