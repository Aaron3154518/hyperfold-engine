use diagnostic::{CatchErr, CombineResults, ErrForEach, ErrorSpan, ErrorTrait, ResultsTrait};
use proc_macro2::{token_stream::IntoIter, Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use std::{collections::VecDeque, env::temp_dir, fs, path::PathBuf};
use syn::{parenthesized, parse_macro_input, spanned::Spanned, token::Trait, PatType, Token};

use crate::{
    codegen::{self as codegen, Crates, Traits},
    component_set::ComponentSet,
    parse::{
        resolve_path, AstCrate, AstFunction, AstItemData, AstItems, AstMod, AstModType, AstUse,
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
    parsing::{ComponentMacroArgs, GlobalMacroArgs, SystemMacroArgs},
    syn::{
        error::{AddSpan, PanicError, PanicResult, SpannedError, SpannedResult, SplitBuildResult},
        parse_tokens,
    },
    traits::{
        Call, Catch, CollectVec, CollectVecInto, FindFrom, GetResult, GetSlice, HandleErr,
        Increment, PushInto, SplitAround,
    },
};

#[derive(Debug)]
pub struct ItemComponent {
    pub data: ItemData,
    pub args: ComponentMacroArgs,
}

#[derive(Clone, Debug)]
pub struct ItemGlobal {
    pub data: ItemData,
    pub args: GlobalMacroArgs,
}

#[derive(Clone, Debug)]
pub struct ItemTrait {
    pub data: ItemData,
    pub g_idx: usize,
}

#[derive(Debug)]
pub struct ItemEvent {
    pub data: ItemData,
    pub state: Option<usize>,
}

#[derive(Debug)]
pub struct ItemState {
    pub data: ItemData,
    pub data_path: ItemPath,
    pub enter_event: usize,
    pub exit_event: usize,
    pub label: usize,
}

#[derive(Debug, Clone)]
pub struct ItemData {
    pub path: ItemPath,
    pub mod_idx: usize,
    pub span: ErrorSpan,
}

impl ItemData {
    pub fn from_ast(cr_idx: usize, mod_idx: usize, item: &AstItemData) -> Self {
        Self {
            path: ItemPath::new(cr_idx, item.path.to_vec()),
            mod_idx,
            span: (&item.span).into(),
        }
    }

    pub fn add_path(mut self, segment: &str) -> Self {
        self.path.path.push(segment.to_string());
        self
    }
}

enum NewItem {
    Component(ItemComponent),
    Global(ItemGlobal),
    Event(ItemEvent),
    State(AstItemData),
    ComponentSet(ComponentSet),
    System(ItemSystem),
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
        let args = comp.args;
        let path = comp.data.path.path.clone();
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
        let args = global.args;
        let path = global.data.path.path.clone();
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
        let path = event.data.path.path.clone();
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
                state: Some(state_idx),
                data: ItemData::from_ast(cr_idx, mod_idx, item).add_path(STATE_ENTER_EVENT),
            }),
            // OnExit
            self.add_event(ItemEvent {
                state: Some(state_idx),
                data: ItemData::from_ast(cr_idx, mod_idx, item).add_path(STATE_EXIT_EVENT),
            }),
            // Label
            self.add_component(ItemComponent {
                args: ComponentMacroArgs::default(),
                data: ItemData::from_ast(cr_idx, mod_idx, item).add_path(STATE_LABEL),
            }),
            {
                let data_path = item.path.to_vec().push_into(STATE_DATA.to_string());
                self.states.push(ItemState {
                    data_path: ItemPath {
                        cr_idx,
                        path: data_path.to_vec(),
                    },
                    enter_event: self.events.len() - 2,
                    exit_event: self.events.len() - 1,
                    label: self.components.len() - 1,
                    data: ItemData::from_ast(cr_idx, mod_idx, item),
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

    fn add_system(&mut self, sys: ItemSystem) -> Symbol {
        let sym = Symbol {
            kind: SymbolType::System(self.systems.len(), sys.span),
            path: sys.path.path.to_vec(),
            public: true,
        };
        self.systems.push(sys);
        sym
    }

    fn add_symbols(
        &mut self,
        crates: &mut Crates,
        f: impl Fn(&Self, &mut Vec<NewItem>, ModInfo) -> SpannedResult<()>,
    ) -> PanicResult<()> {
        let macro_cr_idx = crates.get_crate_index(Crate::Macros);

        // Get symbols
        let all_crates = crates.get_crates();
        let results = crates.iter_except([macro_cr_idx]).map_vec_into(|cr| {
            cr.iter_mods().map_vec_into(|m| {
                let mut items = Vec::new();
                (
                    f(self, &mut items, (m, cr, all_crates))
                        .err()
                        .unwrap_or(Vec::new()),
                    items,
                )
            })
        });

        // Add symbols
        for (cr, results) in crates.iter_except_mut([macro_cr_idx]).zip(results) {
            let cr_idx = cr.idx;
            let mut num_mods = cr.mods.len();
            let mut new_mods = Vec::new();
            for (m, (errs, items)) in cr.iter_mods_mut().zip(results) {
                m.errs.extend(errs);
                for item in items {
                    m.symbols.push(match item {
                        NewItem::Component(c) => self.add_component(c),
                        NewItem::Global(g) => self.add_global(g),
                        NewItem::Event(e) => self.add_event(e),
                        NewItem::State(item) => {
                            let mods = m.add_mod(num_mods, self.add_state(cr_idx, m.idx, &item));
                            num_mods += mods.len();
                            new_mods.extend(mods);
                            continue;
                        }
                        NewItem::ComponentSet(cs) => self.add_component_set(cs),
                        NewItem::System(s) => self.add_system(s),
                    });
                }
            }
            cr.mods.extend(new_mods);
        }

        Ok(())
    }

    pub fn resolve(crates: &mut Crates) -> (Self, Vec<PanicError>) {
        let mut errs = Vec::new();
        let mut items = Items::new();

        let macro_cr_idx = crates.get_crate_index(Crate::Macros);

        // Insert hardcoded symbols
        for sym in HardcodedSymbol::VARIANTS {
            AstCrate::add_hardcoded_symbol(crates, sym).record_errs(&mut errs);
        }

        // Resolve components, globals, events, and states
        items.add_symbols(crates, |_, new_items, (m, cr, crates)| {
            m.items
                .structs_and_enums()
                .do_for_each(|(item, attrs)| {
                    attrs
                        .do_until(|attr| {
                            Ok(
                                match resolve_path(attr.path.to_vec(), (m, cr, crates))
                                    .expect_any_hardcoded()
                                    .discard_symbol()
                                {
                                    Ok(HardcodedSymbol::ComponentMacro) => {
                                        Some(new_items.push(NewItem::Component(ItemComponent {
                                            args: parse_tokens(attr.args.clone())?,
                                            data: ItemData::from_ast(cr.idx, m.idx, item),
                                        })))
                                    }
                                    Ok(HardcodedSymbol::GlobalMacro) => {
                                        Some(new_items.push(NewItem::Global(ItemGlobal {
                                            args: parse_tokens(attr.args.clone())?,
                                            data: ItemData::from_ast(cr.idx, m.idx, item),
                                        })))
                                    }
                                    Ok(HardcodedSymbol::EventMacro) => {
                                        Some(new_items.push(NewItem::Event(ItemEvent {
                                            state: None,
                                            data: ItemData::from_ast(cr.idx, m.idx, item),
                                        })))
                                    }
                                    Ok(HardcodedSymbol::StateMacro) => {
                                        Some(new_items.push(NewItem::State(item.clone())))
                                    }
                                    _ => None,
                                },
                            )
                        })
                        .err_or(())
                })
                .err_or(())
        });

        // Resolve component sets
        items.add_symbols(crates, |_, new_items, (m, cr, crates)| {
            (&m.items.macro_calls)
                .do_for_each(|call| {
                    if resolve_path(call.data.path.to_vec(), (m, cr, crates))
                        .expect_hardcoded(HardcodedSymbol::ComponentsMacro)
                        .is_ok()
                    {
                        new_items.push(NewItem::ComponentSet(ComponentSet::parse(
                            call.args.clone(),
                            (m, cr, crates),
                        )?))
                    }
                    Ok(())
                })
                .err_or(())
        });

        // Insert namespace mod
        // TODO: detect game_crate!() and add there with span
        for cr in crates.iter_except_mut([macro_cr_idx]) {
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
            let num_mods = cr.mods.len();

            if let Some(main) = cr.get_main_mod_mut().record_errs(&mut errs) {
                let mods = main.add_mod(
                    num_mods,
                    NewMod {
                        name: NAMESPACE.to_string(),
                        mods: Vec::new(),
                        uses,
                        symbols: Vec::new(),
                        // TODO: refine span
                        span: main.span,
                    },
                );
                cr.mods.extend(mods);
            }
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
                            args: GlobalMacroArgs::default(),
                            data: ItemData {
                                path: ItemPath::new(cr.idx, gl_path.to_vec()),
                                mod_idx: ns_mod.idx,
                                // TODO: refine the span
                                span: ns_mod.span,
                            },
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
        items.add_symbols(crates, |items, new_items, (m, cr, crates)| {
            (&m.items.functions)
                .do_for_each(|fun| {
                    if let Some(attr) = fun.attrs.iter().find(|attr| {
                        resolve_path(attr.path.to_vec(), (m, cr, crates))
                            .expect_hardcoded(HardcodedSymbol::SystemMacro)
                            .is_ok()
                    }) {
                        new_items.push(NewItem::System(ItemSystem::parse(
                            fun,
                            attr,
                            items,
                            (m, cr, crates),
                        )?));
                    }
                    Ok(())
                })
                .err_or(())
        });

        (items, errs)
    }
}
