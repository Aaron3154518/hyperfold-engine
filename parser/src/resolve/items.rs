use diagnostic::{CatchErr, ErrorSpan};
use proc_macro2::{token_stream::IntoIter, Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use std::{collections::VecDeque, env::temp_dir, fs, path::PathBuf};
use syn::{parenthesized, parse_macro_input, spanned::Spanned, token::Trait, PatType, Token};

use crate::{
    codegen::{self as codegen, Crates, Traits},
    component_set::ComponentSet,
    parse::{
        resolve_path, AstCrate, AstFunction, AstItem, AstItemData, AstItems, AstMod, AstModType,
        AstUse, ComponentSymbol, DiscardSymbol, GlobalSymbol, HardcodedSymbol, ItemPath,
        MatchSymbol, ModInfo, NewMod, Symbol, SymbolType,
    },
    system::ItemSystem,
    utils::{
        constants::NAMESPACE,
        paths::{Crate, EnginePaths, NAMESPACE_USE_STMTS, TRAITS},
    },
};

use shared::{
    constants::{INDEX, INDEX_SEP, STATE_DATA, STATE_ENTER_EVENT, STATE_EXIT_EVENT, STATE_LABEL},
    macros::ExpandEnum,
    match_ok,
    msg_result::{CombineMsgs, MsgTrait, Zip2Msgs, Zip8Msgs},
    parsing::{ComponentMacroArgs, GlobalMacroArgs, SystemMacroArgs},
    syn::{
        error::{AddSpan, SpannedResult, SplitBuildResult},
        parse_tokens,
    },
    traits::{
        Call, Catch, CollectVec, CollectVecInto, FindFrom, GetResult, GetSlice, HandleErr,
        Increment, PushInto, SplitAround,
    },
};

#[derive(Debug)]
pub struct ItemComponentData {
    pub args: ComponentMacroArgs,
}
pub type ItemComponent = Item<ItemComponentData>;

#[derive(Clone, Debug)]
pub struct ItemGlobalData {
    pub args: GlobalMacroArgs,
}
pub type ItemGlobal = Item<ItemGlobalData>;

#[derive(Clone, Debug)]
pub struct ItemTraitData {
    pub g_idx: usize,
}
pub type ItemTrait = Item<ItemTraitData>;

#[derive(Debug)]
pub struct ItemEventData {
    pub state: Option<usize>,
}
pub type ItemEvent = Item<ItemEventData>;

#[derive(Debug)]
pub struct ItemStateData {
    pub data_path: ItemPath,
    pub enter_event: usize,
    pub exit_event: usize,
    pub label: usize,
}
pub type ItemState = Item<ItemStateData>;

#[derive(Debug, Clone)]
pub struct Item<T> {
    pub data: T,
    pub path: ItemPath,
    pub mod_idx: usize,
    pub span: ErrorSpan,
}

// Struct for adding symbols
enum NewSymbol {
    Symbol(Symbol),
    Mod(NewMod),
}

#[derive(Debug)]
pub struct Items {
    pub components: Vec<ItemComponent>,
    pub globals: Vec<ItemGlobal>,
    pub events: Vec<ItemEvent>,
    pub states: Vec<ItemState>,
    pub component_sets: Vec<ComponentSet>,
    pub systems: Vec<ItemSystem>,
}

impl Items {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            globals: Vec::new(),
            events: Vec::new(),
            states: Vec::new(),
            component_sets: Vec::new(),
            systems: Vec::new(),
        }
    }

    fn add_component(&mut self, comp: ItemComponent) -> Symbol {
        let args = comp.data.args;
        let path = comp.path.path.clone();
        self.components.push(comp);
        Symbol {
            kind: SymbolType::Component(ComponentSymbol {
                idx: self.components.len() - 1,
                args,
            }),
            path,
            public: true,
        }
    }

    fn add_global(&mut self, global: ItemGlobal) -> Symbol {
        let args = global.data.args;
        let path = global.path.path.clone();
        self.globals.push(global);
        Symbol {
            kind: SymbolType::Global(GlobalSymbol {
                idx: self.globals.len() - 1,
                args,
            }),
            path,
            public: true,
        }
    }

    fn add_event(&mut self, event: ItemEvent) -> Symbol {
        let path = event.path.path.clone();
        self.events.push(event);
        Symbol {
            kind: SymbolType::Event(self.events.len() - 1),
            path,
            public: true,
        }
    }

    fn add_state(&mut self, cr_idx: usize, mod_idx: usize, item: &AstItemData) -> NewMod {
        let state_idx = self.states.len();
        let symbols = vec![
            // OnEnter
            self.add_event(ItemEvent {
                data: ItemEventData {
                    state: Some(state_idx),
                },
                path: ItemPath {
                    cr_idx,
                    path: item.path.to_vec().push_into(STATE_ENTER_EVENT.to_string()),
                },
                mod_idx,
                span: (&item.span).into(),
            }),
            // OnExit
            self.add_event(ItemEvent {
                data: ItemEventData {
                    state: Some(state_idx),
                },
                path: ItemPath {
                    cr_idx,
                    path: item.path.to_vec().push_into(STATE_EXIT_EVENT.to_string()),
                },
                mod_idx,
                span: (&item.span).into(),
            }),
            // Label
            self.add_component(ItemComponent {
                data: ItemComponentData {
                    args: ComponentMacroArgs::default(),
                },
                path: ItemPath {
                    cr_idx,
                    path: item.path.to_vec().push_into(STATE_LABEL.to_string()),
                },
                mod_idx,
                span: (&item.span).into(),
            }),
            {
                let data_path = item.path.to_vec().push_into(STATE_DATA.to_string());
                self.states.push(ItemState {
                    data: ItemStateData {
                        data_path: ItemPath {
                            cr_idx,
                            path: data_path.to_vec(),
                        },
                        enter_event: self.events.len() - 2,
                        exit_event: self.events.len() - 1,
                        label: self.components.len() - 1,
                    },
                    path: ItemPath {
                        cr_idx,
                        path: item.path.to_vec(),
                    },
                    mod_idx,
                    span: (&item.span).into(),
                });
                Symbol {
                    kind: SymbolType::State(state_idx),
                    path: data_path,
                    public: true,
                }
            },
        ];
        NewMod {
            name: item.ident.to_string(),
            mods: Vec::new(),
            uses: Vec::new(),
            symbols,
            span: (&item.span).into(),
        }
    }

    fn add_component_set(&mut self, cs: ComponentSet) -> Symbol {
        let sym = Symbol {
            kind: SymbolType::ComponentSet(self.component_sets.len()),
            path: cs.path.path.to_vec(),
            public: true,
        };
        self.component_sets.push(cs);
        sym
    }

    fn add_system(&mut self, sys: ItemSystem, fun: &AstItem<AstFunction>) -> Symbol {
        let sym = Symbol {
            kind: SymbolType::System(self.systems.len(), fun.data.sig.span()),
            path: sys.path.path.to_vec(),
            public: true,
        };
        self.systems.push(sys);
        sym
    }

    // fn add_symbols(
    //     &mut self,
    //     errs: &mut Vec<String>,
    //     crates: &mut Crates,
    //     f: impl Fn(&mut Vec<String>, &mut Self, ModInfo) -> Vec<NewSymbol>,
    // ) {
    //     self.add_symbols_result(errs, crates, |a, b, c| f(a, b, c).map_vec_into(|s| Ok(s)))
    // }

    // fn add_symbols_result(
    //     &mut self,
    //     errs: &mut Vec<String>,
    //     crates: &mut Crates,
    //     f: impl Fn(&mut Vec<String>, &mut Self, ModInfo) -> Vec<SpannedResult<NewSymbol>>,
    // ) {
    //     let macro_cr_idx = crates.get_crate_index(Crate::Macros);

    //     // Get symbols
    //     let all_crates = crates.get_crates();
    //     let symbols = crates.iter_except([macro_cr_idx]).map_vec_into(|cr| {
    //         cr.iter_mods()
    //             .map_vec_into(|m| f(errs, self, (m, cr, all_crates)))
    //     });

    //     // Add symbols
    //     for (cr, crate_symbols) in crates.iter_except_mut([macro_cr_idx]).zip(symbols) {
    //         let mut mods = cr.iter_mods_mut();
    //         for mod_symbols in crate_symbols {
    //             if let Some(m) = mods.next(cr) {
    //                 for s in mod_symbols {
    //                     match s {
    //                         Ok(NewSymbol::Symbol(s)) => {
    //                             m.symbols.push(s);
    //                         }
    //                         // Mod iteration order is locked, so adding new mods is safe
    //                         Ok(NewSymbol::Mod(new_mod)) => {
    //                             m.add_mod(new_mod);
    //                         }
    //                         Err(errs) => {
    //                             m.errs.extend(errs);
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    pub fn resolve(crates: &mut Crates) -> (Self, Vec<String>) {
        let mut errs = Vec::new();
        let mut items = Items::new();

        let macro_cr_idx = crates.get_crate_index(Crate::Macros);

        // Insert hardcoded symbols
        for sym in HardcodedSymbol::VARIANTS {
            AstCrate::add_hardcoded_symbol(crates, sym).record_errs(&mut errs);
        }

        // Resolve components, globals, events, and states
        for cr in crates.iter_mut() {
            for (mod_idx, m) in cr.iter_mods_mut().enumerate() {
                for (item, attrs) in m
                    .items
                    .structs
                    .iter()
                    .map(|s| (&s.item, &s.data.attrs))
                    .chain(m.items.enums.iter().map(|e| (&e.item, &e.data.attrs)))
                {
                    // Search through attributes
                    for attr in attrs {
                        match resolve_path(attr.path.to_vec(), (m, cr, crates.get_crates()))
                            .expect_any_hardcoded()
                            .discard_symbol()
                        {
                            Ok(HardcodedSymbol::ComponentMacro) => {
                                if let Some(args) =
                                    parse_tokens(attr.args.clone()).record_errs(&mut m.errs)
                                {
                                    m.add_symbol(items.add_component(ItemComponent {
                                        data: ItemComponentData { args },
                                        path: ItemPath::new(cr.idx, item.path.to_vec()),
                                        mod_idx,
                                        span: (&item.span).into(),
                                    }));
                                }
                                break;
                            }
                            Ok(HardcodedSymbol::GlobalMacro) => {
                                if let Some(args) =
                                    parse_tokens(attr.args.clone()).record_errs(&mut m.errs)
                                {
                                    m.add_symbol(items.add_global(ItemGlobal {
                                        data: ItemGlobalData { args },
                                        path: ItemPath::new(cr.idx, item.path.to_vec()),
                                        mod_idx,
                                        span: (&item.span).into(),
                                    }));
                                }
                                break;
                            }
                            Ok(HardcodedSymbol::EventMacro) => {
                                m.add_symbol(items.add_event(ItemEvent {
                                    data: ItemEventData { state: None },
                                    path: ItemPath::new(cr.idx, item.path.to_vec()),
                                    mod_idx,
                                    span: (&item.span).into(),
                                }));
                                break;
                            }
                            Ok(HardcodedSymbol::StateMacro) => {
                                cr.add_child_mod(mod_idx, items.add_state(cr.idx, mod_idx, item));
                                break;
                            }
                            _ => (),
                        }
                    }
                }
            }
        }

        // Resolve components, globals, events, and states
        // for cr in crates.iter_mut() {
        //     for (mod_idx, m) in cr.iter_mods_mut().enumerate() {
        //         let mut symbols = Vec::new();
        //         for (item, attrs) in m
        //             .items
        //             .structs
        //             .iter()
        //             .map(|s| (&s.item, &s.data.attrs))
        //             .chain(m.items.enums.iter().map(|e| (&e.item, &e.data.attrs)))
        //         {
        //             // Search through attributes
        //             for attr in attrs {
        //                 match resolve_path(attr.path.to_vec(), (m, cr, crates.get_crates()))
        //                     .expect_any_hardcoded()
        //                     .discard_symbol()
        //                 {
        //                     Ok(HardcodedSymbol::ComponentMacro) => {
        //                         if let Some(args) =
        //                             parse_tokens(attr.args.clone()).record_errs(&mut m.errs)
        //                         {
        //                             symbols.push(NewSymbol::Symbol(items.add_component(
        //                                 cr.idx,
        //                                 path.to_vec(),
        //                                 args,
        //                             )));
        //                         }
        //                         break;
        //                     }
        //                     Ok(HardcodedSymbol::GlobalMacro) => {
        //                         if let Some(args) =
        //                             parse_tokens(attr.args.clone()).record_errs(&mut m.errs)
        //                         {
        //                             symbols.push(NewSymbol::Symbol(items.add_global(
        //                                 cr.idx,
        //                                 path.to_vec(),
        //                                 args,
        //                             )));
        //                         }
        //                         break;
        //                     }
        //                     Ok(HardcodedSymbol::EventMacro) => {
        //                         symbols.push(NewSymbol::Symbol(items.add_event(
        //                             cr.idx,
        //                             path.to_vec(),
        //                             None,
        //                         )));
        //                         break;
        //                     }
        //                     Ok(HardcodedSymbol::StateMacro) => {
        //                         symbols.push(NewSymbol::Mod(items.add_state(
        //                             cr.idx,
        //                             ident.to_string(),
        //                             path.to_vec(),
        //                         )));
        //                         break;
        //                     }
        //                     _ => (),
        //                 }
        //             }
        //         }
        //     }
        // }

        // Resolve component sets
        for cr in crates.iter_mut() {
            for (mod_idx, m) in cr.iter_mods_mut().enumerate() {
                for call in &m.items.macro_calls {
                    if let Some(symbol) =
                        resolve_path(call.item.path.to_vec(), (m, cr, crates.get_crates()))
                            .expect_hardcoded(HardcodedSymbol::ComponentsMacro)
                            .ok()
                            .and_then(|_| {
                                ComponentSet::parse(
                                    call.data.args.clone(),
                                    (m, cr, crates.get_crates()),
                                )
                                .map(|cs| items.add_component_set(cs))
                                .record_errs(&mut m.errs)
                            })
                    {
                        m.add_symbol(symbol);
                    }
                }
            }
        }

        // Insert namespace mod
        // TODO: detect game_crate!() and add there with span
        for cr in crates.iter_except_mut([macro_cr_idx]) {
            cr.get_main_mod()
                .and_then(|main| {
                    // Traits
                    let uses = cr
                        .deps
                        .iter()
                        .map(|(_, alias)| AstUse {
                            ident: alias.to_string(),
                            first: alias.to_string(),
                            path: vec![alias.to_string()],
                            public: true,
                        })
                        .collect();

                    cr.add_child_mod(
                        main.idx,
                        NewMod {
                            name: NAMESPACE.to_string(),
                            mods: Vec::new(),
                            uses,
                            symbols: Vec::new(),
                            span: main.span,
                        },
                    )
                })
                .record_errs(&mut errs);
        }

        // Insert trait symbols
        for tr in &*TRAITS {
            let path = tr.main_trait.full_path();
            let gl_path = tr.global.full_path();

            let idx = items.globals.len();
            for cr in crates.iter_except_mut([macro_cr_idx]) {
                if let Some(ns_mod) = cr.find_mod(gl_path.slice_to(-1)).record_errs(&mut errs) {
                    if cr.idx == 0 {
                        // Reference global in entry crate
                        items.globals.push(ItemGlobal {
                            data: ItemGlobalData {
                                args: GlobalMacroArgs::default(),
                            },
                            path: ItemPath::new(0, gl_path.to_vec()),
                            mod_idx: ns_mod.idx,
                            span: ns_mod.span,
                        });
                    }
                    // Add globals to entry crates
                    let g_sym = GlobalSymbol {
                        idx,
                        args: GlobalMacroArgs::default(),
                    };
                    if cr.idx == 0 {
                        cr.add_symbol(Symbol {
                            kind: SymbolType::Global(g_sym),
                            path: gl_path.to_vec(),
                            public: true,
                        })
                        .record_errs(&mut errs);
                    }
                    // Add traits to all crates
                    cr.add_symbol(Symbol {
                        kind: SymbolType::Trait(g_sym),
                        path: path.to_vec(),
                        public: true,
                    })
                    .record_errs(&mut errs);
                }
            }
        }

        // Resolve systems
        for cr in crates.iter_mut() {
            for (mod_idx, m) in cr.iter_mods_mut().enumerate() {
                for fun in &m.items.functions {
                    if let Some(symbol) = fun.data.attrs.iter().find_map(|attr| {
                        resolve_path(attr.path.to_vec(), (m, cr, crates.get_crates()))
                            .expect_hardcoded(HardcodedSymbol::SystemMacro)
                            .ok()
                            .and_then(|sym| {
                                ItemSystem::parse(fun, attr, &items, (m, cr, crates.get_crates()))
                                    .map(|sys| items.add_system(sys, fun))
                                    .record_errs(&mut m.errs)
                            })
                    }) {
                        m.add_symbol(symbol)
                    }
                }
            }
        }

        (items, errs)
    }
}
