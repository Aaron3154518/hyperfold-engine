use proc_macro2::{token_stream::IntoIter, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use std::{collections::VecDeque, env::temp_dir, fs, path::PathBuf};
use syn::{parenthesized, parse_macro_input, spanned::Spanned, token::Trait, PatType, Token};

use crate::{
    codegen::{self as codegen, Crates, Traits},
    component_set::ComponentSet,
    parse::{
        resolve_path, AstCrate, AstFunction, AstItem, AstItems, AstMod, AstModType, AstUse,
        ComponentSymbol, DiscardSymbol, GlobalSymbol, HardcodedSymbol, ItemPath, MatchSymbol,
        ModInfo, NewMod, Symbol, SymbolType,
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
    syn::{error::SplitBuildResult, parse_tokens},
    traits::{
        Call, Catch, CollectVec, CollectVecInto, FindFrom, GetResult, HandleErr, Increment,
        PushInto, SplitAround,
    },
};

#[derive(Debug)]
pub struct ItemComponent {
    pub path: ItemPath,
    pub args: ComponentMacroArgs,
}

#[derive(Clone, Debug)]
pub struct ItemGlobal {
    pub path: ItemPath,
    pub args: GlobalMacroArgs,
}

#[derive(Clone, Debug)]
pub struct ItemTrait {
    pub path: ItemPath,
    pub g_idx: usize,
}

#[derive(Debug)]
pub struct ItemEvent {
    pub path: ItemPath,
    pub state: Option<usize>,
}

#[derive(Debug)]
pub struct ItemState {
    pub path: ItemPath,
    pub data_path: ItemPath,
    pub enter_event: usize,
    pub exit_event: usize,
    pub label: usize,
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

    fn add_component(
        &mut self,
        cr_idx: usize,
        path: Vec<String>,
        args: ComponentMacroArgs,
    ) -> Symbol {
        self.components.push(ItemComponent {
            path: ItemPath {
                cr_idx,
                path: path.to_vec(),
            },
            args,
        });
        Symbol {
            kind: SymbolType::Component(ComponentSymbol {
                idx: self.components.len() - 1,
                args,
            }),
            path,
            public: true,
        }
    }

    fn add_global(&mut self, cr_idx: usize, path: Vec<String>, args: GlobalMacroArgs) -> Symbol {
        self.globals.push(ItemGlobal {
            path: ItemPath {
                cr_idx,
                path: path.to_vec(),
            },
            args,
        });
        Symbol {
            kind: SymbolType::Global(GlobalSymbol {
                idx: self.globals.len() - 1,
                args,
            }),
            path,
            public: true,
        }
    }

    fn add_event(&mut self, cr_idx: usize, path: Vec<String>, state: Option<usize>) -> Symbol {
        self.events.push(ItemEvent {
            path: ItemPath {
                cr_idx,
                path: path.to_vec(),
            },
            state,
        });
        Symbol {
            kind: SymbolType::Event(self.events.len() - 1),
            path,
            public: true,
        }
    }

    fn add_state(&mut self, cr_idx: usize, name: String, path: Vec<String>) -> NewMod {
        let state_idx = self.states.len();
        let symbols = vec![
            // OnEnter
            self.add_event(
                cr_idx,
                path.to_vec().push_into(STATE_ENTER_EVENT.to_string()),
                Some(state_idx),
            ),
            // OnExit
            self.add_event(
                cr_idx,
                path.to_vec().push_into(STATE_EXIT_EVENT.to_string()),
                Some(state_idx),
            ),
            // Label
            self.add_component(
                cr_idx,
                path.to_vec().push_into(STATE_LABEL.to_string()),
                ComponentMacroArgs::default(),
            ),
            {
                let data_path = path.to_vec().push_into(STATE_DATA.to_string());
                self.states.push(ItemState {
                    path: ItemPath {
                        cr_idx,
                        path: path.to_vec(),
                    },
                    data_path: ItemPath {
                        cr_idx,
                        path: data_path.to_vec(),
                    },
                    enter_event: self.events.len() - 2,
                    exit_event: self.events.len() - 1,
                    label: self.components.len() - 1,
                });
                Symbol {
                    kind: SymbolType::State(state_idx),
                    path: data_path,
                    public: true,
                }
            },
        ];
        NewMod {
            name,
            mods: Vec::new(),
            uses: Vec::new(),
            symbols,
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

    fn add_symbols(
        &mut self,
        errs: &mut Vec<String>,
        crates: &mut Crates,
        f: impl Fn(&mut Vec<String>, &mut Self, ModInfo) -> Vec<NewSymbol>,
    ) {
        let macro_cr_idx = crates.get_crate_index(Crate::Macros);

        // Get symbols
        let all_crates = crates.get_crates();
        let symbols = crates.iter_except([macro_cr_idx]).map_vec_into(|cr| {
            cr.iter_mods()
                .map_vec_into(|m| f(errs, self, (m, cr, all_crates)))
        });

        // Add symbols
        for (cr, crate_symbols) in crates.iter_except_mut([macro_cr_idx]).zip(symbols) {
            let mut mods = cr.iter_mods_mut();
            for mod_symbols in crate_symbols {
                if let Some(m) = mods.next(cr) {
                    let mod_symbols = mod_symbols.filter_map_vec_into(|s| match s {
                        NewSymbol::Symbol(s) => Some(s),
                        // Mod iteration order is locked, so adding new mods is safe
                        NewSymbol::Mod(new_mod) => {
                            m.add_mod(new_mod);
                            None
                        }
                    });
                    m.symbols.extend(mod_symbols);
                }
            }
        }
    }

    pub fn resolve(crates: &mut Crates) -> (Self, Vec<String>) {
        let mut errs = Vec::new();
        let mut items = Items::new();

        let macro_cr_idx = crates.get_crate_index(Crate::Macros);

        // Insert hardcoded symbols
        for sym in HardcodedSymbol::VARIANTS {
            AstCrate::add_hardcoded_symbol(crates, sym).record_errs(&mut errs);
        }

        // Resolve components, globals, events, and states
        items.add_symbols(&mut errs, crates, |errs, items, mod_info| {
            let (m, cr, crates) = mod_info;
            let mut symbols = Vec::new();
            for (ident, path, attrs) in m
                .items
                .structs
                .iter()
                .map(|s| (&s.ident, &s.path, &s.data.attrs))
                .chain(
                    m.items
                        .enums
                        .iter()
                        .map(|e| (&e.ident, &e.path, &e.data.attrs)),
                )
            {
                // Search through attributes
                for attr in attrs {
                    match resolve_path(attr.path.to_vec(), mod_info)
                        .expect_any_hardcoded()
                        .discard_symbol()
                    {
                        Ok(HardcodedSymbol::ComponentMacro) => {
                            if let Some(args) = parse_tokens(attr.args.clone())
                                .take_spanned_errs(&mut m.errs)
                                .record_errs(errs)
                            {
                                symbols.push(NewSymbol::Symbol(items.add_component(
                                    cr.idx,
                                    path.to_vec(),
                                    args,
                                )));
                            }
                            break;
                        }
                        Ok(HardcodedSymbol::GlobalMacro) => {
                            if let Some(args) = parse_tokens(attr.args.clone())
                                .take_spanned_errs(&mut m.errs)
                                .record_errs(errs)
                            {
                                symbols.push(NewSymbol::Symbol(items.add_global(
                                    cr.idx,
                                    path.to_vec(),
                                    args,
                                )));
                            }
                            break;
                        }
                        Ok(HardcodedSymbol::EventMacro) => {
                            symbols.push(NewSymbol::Symbol(items.add_event(
                                cr.idx,
                                path.to_vec(),
                                None,
                            )));
                            break;
                        }
                        Ok(HardcodedSymbol::StateMacro) => {
                            symbols.push(NewSymbol::Mod(items.add_state(
                                cr.idx,
                                ident.to_string(),
                                path.to_vec(),
                            )));
                            break;
                        }
                        _ => (),
                    }
                }
            }
            symbols
        });

        // Resolve component sets
        items.add_symbols(&mut errs, crates, |errs, items, mod_info| {
            mod_info.0.items.macro_calls.filter_map_vec(|call| {
                resolve_path(call.path.to_vec(), mod_info)
                    .expect_hardcoded(HardcodedSymbol::ComponentsMacro)
                    .ok()
                    .and_then(|_| {
                        ComponentSet::parse(call.data.args.clone(), mod_info)
                            .handle_err(|es| errs.extend(es))
                            .map(|cs| NewSymbol::Symbol(items.add_component_set(cs)))
                    })
            })
        });

        // Insert namespace mod
        for cr in crates.iter_except_mut([macro_cr_idx]) {
            let path = cr.main.path.to_vec();
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

            cr.add_mod(
                path,
                NewMod {
                    name: NAMESPACE.to_string(),
                    mods: Vec::new(),
                    uses,
                    symbols: Vec::new(),
                },
            )
            .record_errs(&mut errs);
        }

        // Insert trait symbols
        for tr in &*TRAITS {
            let path = tr.main_trait.full_path();
            let gl_path = tr.global.full_path();
            // Reference global in entry crate
            items.globals.push(ItemGlobal {
                path: ItemPath {
                    cr_idx: 0,
                    path: gl_path.to_vec(),
                },
                args: GlobalMacroArgs::default(),
            });
            for cr in crates.iter_except_mut([macro_cr_idx]) {
                let idx = items.globals.len() - 1;
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

        // Resolve systems
        items.add_symbols(&mut errs, crates, |errs, items, mod_info| {
            let (m, cr, crates) = mod_info;
            m.items.functions.iter().filter_map_vec_into(|fun| {
                fun.data.attrs.iter().find_map(|attr| {
                    resolve_path(attr.path.to_vec(), mod_info)
                        .expect_hardcoded(HardcodedSymbol::SystemMacro)
                        .ok()
                        .and_then(|sym| {
                            ItemSystem::parse(fun, attr, &items, mod_info)
                                .handle_err(|es| errs.extend(es))
                                .map(|sys| NewSymbol::Symbol(items.add_system(sys, fun)))
                        })
                })
            })
        });

        (items, errs)
    }
}
