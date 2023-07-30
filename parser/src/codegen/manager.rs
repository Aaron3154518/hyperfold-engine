use proc_macro2::TokenStream;
use quote::quote;

use shared::{
    match_ok,
    msg_result::{Zip5Msgs, Zip6Msgs},
    traits::CollectVecInto,
};

use crate::{
    component_set::ComponentSet,
    resolve::Items,
    system::{codegen_systems, SystemsCodegenResult},
    utils::{
        idents::{CodegenIdents, CODEGEN_IDENTS},
        paths::{Crate, EngineGlobalPaths, ENGINE_GLOBALS, ENGINE_PATHS, ENGINE_TRAITS},
        MsgResult,
    },
};

use super::Crates;

pub fn manager_def() -> TokenStream {
    let CodegenIdents {
        manager,
        globals,
        components,
        events,
        event_enum,
        event_enum_len,
        cfoo_var: comps_var,
        gfoo_var: globals_var,
        efoo_var: events_var,
        stack_var,
        services_var,
        ..
    } = &*CODEGEN_IDENTS;

    quote!(
        pub struct #manager {
            #comps_var: #components,
            #globals_var: #globals,
            #events_var: #events,
            #stack_var: Vec<std::collections::VecDeque<(#event_enum, usize)>>,
            #services_var: [Vec<Box<dyn Fn(&mut #components, &mut #globals, &mut #events)>>; #event_enum_len]
        }
    )
}

// Function to set initial events on each tick
fn init_events_fn(cr_idx: usize, items: &Items, crates: &Crates) -> MsgResult<TokenStream> {
    let CodegenIdents {
        events,
        efoo_var: events_var,
        ..
    } = &*CODEGEN_IDENTS;

    let add_event = crates.get_syn_path(cr_idx, &ENGINE_TRAITS.add_event);

    let core_events = crates.get_syn_path(cr_idx, &ENGINE_PATHS.core_events);
    let core_update = crates.get_syn_path(cr_idx, &ENGINE_PATHS.core_update);
    let core_pre_render = crates.get_syn_path(cr_idx, &ENGINE_PATHS.core_pre_render);
    let core_render = crates.get_syn_path(cr_idx, &ENGINE_PATHS.core_render);

    match_ok!(
        Zip5Msgs,
        add_event,
        core_events,
        core_update,
        core_pre_render,
        core_render,
        {
            quote!(
                fn init_events(&self, ts: u32) -> #events {
                    let mut #events_var = #events::new();
                    #add_event::new_event(&mut #events_var, #core_events);
                    #add_event::new_event(&mut #events_var, #core_update(ts));
                    #add_event::new_event(&mut #events_var, #core_pre_render);
                    #add_event::new_event(&mut #events_var, #core_render);
                    #events_var
                }
            )
        }
    )
}

pub fn manager_impl(cr_idx: usize, items: &Items, crates: &Crates) -> MsgResult<TokenStream> {
    let CodegenIdents {
        manager,
        globals,
        components,
        events,
        event_enum,
        event_enum_len,
        cfoo_var,
        gfoo_var,
        efoo_var,
        stack_var,
        services_var,
        exiting_state_var,
        ..
    } = &*CODEGEN_IDENTS;

    let result = codegen_systems(cr_idx, items, crates);
    let init_events = init_events_fn(cr_idx, items, crates);
    let path_to_engine = crates.get_named_crate_syn_path(cr_idx, Crate::Engine);
    let component_set_fns =
        ComponentSet::codegen_get_keys_fns(cr_idx, &items.component_sets, crates);
    let global_paths = ENGINE_GLOBALS.get_global_vars(crates, cr_idx);
    let manager_trait = crates.get_syn_path(cr_idx, &ENGINE_PATHS.manager_trait);

    match_ok!(
        Zip6Msgs,
        result,
        init_events,
        path_to_engine,
        component_set_fns,
        global_paths,
        manager_trait,
        {
            let SystemsCodegenResult {
                init_systems,
                systems,
                system_events,
            } = result;
            let EngineGlobalPaths {
                c_foo: g_c_foo,
                e_foo: g_e_foo,
                entity_trash: g_entity_trash,
                event: g_event,
                renderer: g_renderer,
                camera: g_camera,
                screen: g_screen,
            } = global_paths;
            quote!(
                impl #manager {
                    #(#component_set_fns)*

                    fn add_system(&mut self, e: #event_enum, f: Box<dyn Fn(&mut #components, &mut #globals, &mut #events)>) {
                        self.#services_var[e as usize].push(f);
                    }

                    fn add_systems(&mut self) {
                        #(self.add_system(#event_enum::#system_events, Box::new(#systems));)*
                    }
                }

                impl #manager {
                    fn init(&mut self) {
                        #(#init_systems;)*
                        let events = std::mem::replace(&mut self.#gfoo_var.#g_e_foo, #events::new());
                        self.add_events(events);
                        self.update_entities();
                    }

                    fn tick(&mut self, ts: u32) {
                        self.#gfoo_var
                            .#g_event
                            .update(ts, &self.#gfoo_var.#g_camera.0, &self.#gfoo_var.#g_screen.0);
                        self.#gfoo_var.#g_renderer.clear();
                        self.add_events(self.init_events(ts));
                        while !self.#stack_var.is_empty() {
                            self.process_next_event();
                            if let Some((e, i, n)) = self
                                .#stack_var
                                .last_mut()
                                .and_then(|queue| queue.front_mut())
                                .and_then(|(e, i)| {
                                    let v_s = &self.#services_var[*e as usize];
                                    v_s.get(*i).map(|_| {
                                        let vals = (e.clone(), i.clone(), v_s.len());
                                        *i += 1;
                                        vals
                                    })
                                })
                            {
                                self.#gfoo_var.#g_e_foo = #events::new();
                                if let Some(s) = self.#services_var[e as usize].get(i) {
                                    (s)(&mut self.#cfoo_var, &mut self.#gfoo_var, &mut self.#efoo_var);
                                }
                                if i + 1 >= n {
                                    self.pop();
                                }
                                let events = std::mem::replace(&mut self.#gfoo_var.#g_e_foo, #events::new());
                                self.add_events(events);
                                self.update_entities();
                            } else {
                                self.pop();
                            }
                        }
                        self.#gfoo_var.#g_renderer.present();
                    }

                    fn update_entities(&mut self) {
                        self.#cfoo_var.remove(&mut self.#gfoo_var.#g_entity_trash);
                        self.#cfoo_var.append(&mut self.#gfoo_var.#g_c_foo);
                    }

                    #init_events

                    fn add_events(&mut self, mut em: #events) {
                        if em.has_events() {
                            self.#efoo_var.append(&mut em);
                            self.#stack_var.push(em.get_events());
                        }
                    }

                    fn pop(&mut self) {
                        if let Some((e, _)) = self.#stack_var.last_mut().and_then(|queue| queue.pop_front()) {
                            self.#efoo_var.pop(e);
                        };
                        if self.#stack_var.last_mut().is_some_and(|queue| queue.is_empty()) {
                            self.#stack_var.pop();
                        }
                    }

                    fn process_next_event(&mut self) {
                        if let Some(queue) = self.#stack_var.last_mut() {
                            match queue.front().map(|(e, i)| (e.enters_state(), i)) {
                                Some((Some(s), i)) if *i == 0 => match self.#efoo_var.#exiting_state_var {
                                    true => self.#efoo_var.finish_state_change(s),
                                    false => match self.#efoo_var.start_state_change() {
                                        Some(e) => queue.push_front((e, 0)),
                                        None => self.#efoo_var.finish_state_change(s),
                                    },
                                },
                                _ => (),
                            }
                        }
                    }
                }

                impl #manager_trait for #manager {
                    fn new() -> Self {
                        let mut s = Self {
                            #cfoo_var: #components::new(),
                            #gfoo_var: #globals::new(),
                            #efoo_var: #events::new(),
                            #stack_var: Vec::new(),
                            #services_var: std::array::from_fn(|_| Vec::new())
                        };
                        s.init();
                        s.add_systems();
                        s
                    }

                    fn run(&mut self) {
                        const FPS: u32 = 60;
                        const FRAME_TIME: u32 = 1000 / FPS;
                        let mut t = unsafe { #path_to_engine::sdl2::SDL_GetTicks() };
                        let mut dt;
                        let mut tsum: u64 = 0;
                        let mut tcnt: u64 = 0;
                        while !self.#gfoo_var.#g_event.quit {
                            dt = unsafe { #path_to_engine::sdl2::SDL_GetTicks() } - t;
                            t += dt;
                            self.tick(dt);
                            dt = unsafe { #path_to_engine::sdl2::SDL_GetTicks() } - t;
                            tsum += dt as u64;
                            tcnt += 1;
                            if dt < FRAME_TIME {
                                unsafe { #path_to_engine::sdl2::SDL_Delay(FRAME_TIME - dt) };
                            }
                        }
                        println!("Average Frame Time: {}ms", tsum as f64 / tcnt as f64);
                    }
                }
            )
        }
    )
}
