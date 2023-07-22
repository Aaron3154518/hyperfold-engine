use proc_macro2::{token_stream::IntoIter, TokenStream, TokenTree};
use quote::quote;
use quote::ToTokens;
use shared::msg_result::MsgResult;
use shared::msg_result::MsgTrait;
use shared::msg_result::Zip8Msgs;
use std::{collections::VecDeque, env::temp_dir, fs, path::PathBuf};
use syn::{
    parenthesized, parse_macro_input, spanned::Spanned, token::Trait, Error, PatType, Token,
};

use crate::utils::CatchErr;
use crate::{
    codegen::{self as codegen, Crates, Traits},
    component_set::ComponentSet,
    parse::{
        resolve_path, AstCrate, AstMod, AstUse, ComponentSymbol, DiscardSymbol, GlobalSymbol,
        HardcodedSymbol, ItemPath, MatchSymbol, Symbol, SymbolType,
    },
    system::ItemSystem,
    utils::{
        constants::NAMESPACE,
        paths::{Crate, EnginePaths, NAMESPACE_USE_STMTS, TRAITS},
        Msg,
    },
};

use shared::{
    constants::{INDEX, INDEX_SEP},
    macros::ExpandEnum,
    match_ok,
    msg_result::{CombineMsgs, Zip2Msgs},
    parsing::{ComponentMacroArgs, GlobalMacroArgs, SystemMacroArgs},
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
}

#[derive(Debug)]
pub struct Items {
    pub components: Vec<ItemComponent>,
    pub globals: Vec<ItemGlobal>,
    pub events: Vec<ItemEvent>,
    pub component_sets: Vec<ComponentSet>,
    pub systems: Vec<ItemSystem>,
}

impl Items {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            globals: Vec::new(),
            events: Vec::new(),
            component_sets: Vec::new(),
            systems: Vec::new(),
        }
    }

    pub fn resolve(crates: &mut Crates) -> (Self, Vec<Msg>) {
        let mut errs = Vec::new();
        let mut items = Items::new();

        let main_cr_idx = crates.get_crate_index(Crate::Main);
        let macro_cr_idx = crates.get_crate_index(Crate::Macros);

        // Insert hardcoded symbols
        for sym in HardcodedSymbol::VARIANTS {
            AstCrate::add_hardcoded_symbol(crates, sym).record_errs(&mut errs);
        }

        // Resolve components, globals, and events
        let symbols = crates.iter_except([macro_cr_idx]).map_vec_into(|cr| {
            cr.iter_mods().map_vec_into(|m| {
                m.items
                    .structs
                    .iter()
                    .map(|s| (&s.path, &s.data.attrs))
                    .chain(m.items.enums.iter().map(|e| (&e.path, &e.data.attrs)))
                    .filter_map_vec_into(|(path, attrs)| {
                        // Search through attributes
                        attrs
                            .iter()
                            .find_map(|attr| {
                                resolve_path(attr.path.to_vec(), (m, cr, crates.get_crates()))
                                    .expect_any_hardcoded()
                                    .discard_symbol()
                                    .ok()
                                    .and_then(|hard_sym| match hard_sym {
                                        HardcodedSymbol::ComponentMacro => {
                                            let args = ComponentMacroArgs::from(&attr.args);
                                            items.components.push(ItemComponent {
                                                path: ItemPath {
                                                    cr_idx: cr.idx,
                                                    path: path.to_vec(),
                                                },
                                                args,
                                            });
                                            Some(SymbolType::Component(ComponentSymbol {
                                                idx: items.components.len() - 1,
                                                args,
                                            }))
                                        }
                                        HardcodedSymbol::GlobalMacro => {
                                            let args = GlobalMacroArgs::from(&attr.args);
                                            items.globals.push(ItemGlobal {
                                                path: ItemPath {
                                                    cr_idx: cr.idx,
                                                    path: path.to_vec(),
                                                },
                                                args,
                                            });
                                            Some(SymbolType::Global(GlobalSymbol {
                                                idx: items.globals.len() - 1,
                                                args,
                                            }))
                                        }
                                        HardcodedSymbol::EventMacro => {
                                            items.events.push(ItemEvent {
                                                path: ItemPath {
                                                    cr_idx: cr.idx,
                                                    path: path.to_vec(),
                                                },
                                            });
                                            Some(SymbolType::Event(items.events.len() - 1))
                                        }
                                        _ => None,
                                    })
                            })
                            .map(|kind| Symbol {
                                kind,
                                path: path.to_vec(),
                                public: true,
                            })
                    })
            })
        });

        // Insert components, globals, and events
        for (cr, crate_symbols) in crates.iter_except_mut([macro_cr_idx]).zip(symbols) {
            let mut mods = cr.iter_mods_mut();
            for mod_symbols in crate_symbols {
                if let Some(m) = mods.next(cr) {
                    m.symbols.extend(mod_symbols)
                }
            }
        }

        // Resolve component sets
        let symbols = crates.iter_except([macro_cr_idx]).map_vec_into(|cr| {
            cr.iter_mods().map_vec_into(|m| {
                m.items.macro_calls.filter_map_vec(|call| {
                    resolve_path(call.path.to_vec(), (m, cr, crates.get_crates()))
                        .expect_hardcoded(HardcodedSymbol::ComponentsMacro)
                        .ok()
                        .and_then(|_| {
                            ComponentSet::parse(
                                call.data.args.clone(),
                                (m, cr, crates.get_crates()),
                            )
                            .handle_err(|es| errs.extend(es))
                            .map(|cs| {
                                let sym = Symbol {
                                    kind: SymbolType::ComponentSet(items.component_sets.len()),
                                    path: cs.path.path.to_vec(),
                                    public: true,
                                };
                                items.component_sets.push(cs);
                                sym
                            })
                        })
                })
            })
        });

        // Insert component set symbols
        for (cr, crate_symbols) in crates.iter_except_mut([macro_cr_idx]).zip(symbols) {
            let mut mods = cr.iter_mods_mut();
            for mod_symbols in crate_symbols {
                if let Some(m) = mods.next(cr) {
                    m.symbols.extend(mod_symbols);
                }
            }
        }

        // Insert namespace mod
        for cr in crates.iter_except_mut([macro_cr_idx]) {
            let path = cr.main.path.to_vec();
            // Traits
            let uses = cr
                .deps
                .iter()
                .map(|(_, alias)| AstUse {
                    ident: alias.to_string(),
                    path: vec![alias.to_string()],
                    public: true,
                })
                .collect();

            if let Some(m) = cr
                .add_mod(path.push_into(NAMESPACE.to_string()))
                .record_errs(&mut errs)
            {
                m.uses = uses;
            }
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
                args: GlobalMacroArgs::from(&Vec::new()),
            });
            for cr in crates.iter_except_mut([macro_cr_idx]) {
                let idx = items.globals.len() - 1;
                // Add globals to entry crates
                let g_sym = GlobalSymbol {
                    idx,
                    args: GlobalMacroArgs::from(&Vec::new()),
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
        let symbols = crates.iter_except([macro_cr_idx]).map_vec_into(|cr| {
            cr.iter_mods().map_vec_into(|m| {
                m.items.functions.iter().filter_map_vec_into(|fun| {
                    fun.data.attrs.iter().find_map(|attr| {
                        resolve_path(attr.path.to_vec(), (m, cr, crates.get_crates()))
                            .expect_hardcoded(HardcodedSymbol::SystemMacro)
                            .ok()
                            .and_then(|sym| {
                                ItemSystem::parse(fun, attr, &items, (m, cr, crates.get_crates()))
                                    .handle_err(|es| errs.extend(es))
                                    .map(|sys| {
                                        let sym = Symbol {
                                            kind: SymbolType::System(
                                                items.systems.len(),
                                                fun.data.sig.span(),
                                            ),
                                            path: sys.path.path.to_vec(),
                                            public: true,
                                        };
                                        items.systems.push(sys);
                                        sym
                                    })
                            })
                    })
                })
            })
        });

        // Insert system symbols
        for (cr, crate_symbols) in crates.iter_except_mut([macro_cr_idx]).zip(symbols) {
            let mut mods = cr.iter_mods_mut();
            for mod_symbols in crate_symbols {
                if let Some(m) = mods.next(cr) {
                    m.symbols.extend(mod_symbols)
                }
            }
        }

        (items, errs)
    }
}
