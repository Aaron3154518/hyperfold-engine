use proc_macro2::TokenStream;
use quote::quote;

use shared::util::JoinMapInto;

use crate::{
    codegen2::{systems, CodegenIdents, CODEGEN_IDENTS},
    match_ok,
    resolve::{
        util::{CombineMsgs, MsgsResult, Zip2Msgs, Zip4Msgs, Zip5Msgs, Zip7Msgs},
        Crate, Items, ENGINE_PATHS, ENGINE_TRAITS,
    },
};

use super::{component_sets, systems::SystemsCodegenResult, Crates};

pub fn manager_def() -> TokenStream {
    let CodegenIdents {
        manager,
        globals,
        components,
        events,
        event_enum,
        event_enum_len,
        comps_var,
        globals_var,
        events_var,
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
fn init_events_fn(cr_idx: usize, items: &Items, crates: &Crates) -> MsgsResult<TokenStream> {
    let CodegenIdents {
        events, events_var, ..
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

// struct GlobalVars {}

// fn global_vars() -> MsgsResult<GlobalVars> {}

// TODO: harcoded paths: events, sdl funcs (path to engine)
pub fn manager_impl(cr_idx: usize, items: &Items, crates: &Crates) -> MsgsResult<TokenStream> {
    let CodegenIdents {
        manager,
        globals,
        components,
        events,
        event_enum,
        event_enum_len,
        comps_var,
        globals_var,
        events_var,
        stack_var,
        services_var,
        ..
    } = &*CODEGEN_IDENTS;

    let result = systems(cr_idx, items, crates);
    let init_events = init_events_fn(cr_idx, items, crates);
    let path_to_engine = crates.get_named_crate_syn_path(cr_idx, Crate::Engine);
    let component_set_fns = component_sets(cr_idx, &items.component_sets, crates);

    match_ok!(
        Zip4Msgs,
        result,
        init_events,
        path_to_engine,
        component_set_fns,
        {
            let SystemsCodegenResult {
                init_systems,
                systems,
                system_events,
            } = result;
            quote!(
                impl #manager {
                    pub fn new() -> Self {
                        let mut s = Self {
                            #comps_var: #components::new(),
                            #globals_var: #globals::new(),
                            #events_var: #events::new(),
                            #stack_var: Vec::new(),
                            #services_var: std::array::from_fn(|_| Vec::new())
                        };
                        s.init();
                        self.add_systems();
                        s
                    }

                    fn init(&mut self) {
                        #(#init_systems;)*
                        self.post_tick();
                    }

                    fn add_system(&mut self, e: #event_enum, f: Box<dyn Fn(&mut #components, &mut #globals, &mut #events)>) {
                        self.#services_var[e as usize].push(f);
                    }

                    #(#component_set_fns)*

                    fn add_systems(&mut self) {
                        #(self.add_system(#event_enum::#system_events, Box::new(#systems));)*
                    }

                    pub fn run(&mut self) {
                        const FPS: u32 = 60;
                        const FRAME_TIME: u32 = 1000 / FPS;
                        let mut t = unsafe { #path_to_engine::sdl2::SDL_GetTicks() };
                        let mut dt;
                        let mut tsum: u64 = 0;
                        let mut tcnt: u64 = 0;
                        while !self.#globals_var.g1_5.quit {
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

                    fn tick(&mut self, ts: u32) {
                        self.#globals_var
                            .g1_5
                            .update(ts, &self.#globals_var.g1_4.0, &self.#globals_var.g1_3.0);
                        self.#globals_var.g1_1.clear();
                        self.add_events(self.init_events(ts));
                        while !self.#stack_var.is_empty() {
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
                                if i + 1 >= n {
                                    self.pop();
                                }
                                self.#globals_var.g0_1 = #events::new();
                                if let Some(s) = self.#services_var[e as usize].get(i) {
                                    (s)(&mut self.#comps_var, &mut self.#globals_var, &mut self.#events_var);
                                }
                                if i + 1 >= n {
                                    self.#events_var.pop(e);
                                }
                                let #events_var = std::mem::replace(&mut self.#globals_var.g0_1, #events::new());
                                self.add_events(#events_var);
                            } else {
                                self.pop();
                            }
                        }
                        self.#globals_var.g1_1.present();
                        self.post_tick();
                    }

                    fn post_tick(&mut self) {
                        self.#comps_var.remove(&mut self.#globals_var.g1_0);
                        self.#comps_var.append(&mut self.#globals_var.g0_0);
                    }

                    #init_events

                    fn add_events(&mut self, mut em: #events) {
                        if em.has_events() {
                            self.#events_var.append(&mut em);
                            self.#stack_var.push(em.get_events());
                        }
                    }

                    fn pop(&mut self) {
                        if self.#stack_var.last_mut().is_some_and(|queue| {
                            queue.pop_front();
                            queue.is_empty()
                        }) {
                            self.#stack_var.pop();
                        }
                    }
                }
            )
        }
    )
}
