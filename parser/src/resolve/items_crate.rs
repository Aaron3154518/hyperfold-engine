use std::{collections::VecDeque, path::PathBuf};

use crate::{
    codegen2::CratePaths,
    // codegen::component_set::{self},
    parse::{
        AstCrate, AstMod, AstUse, DiscardSymbol, HardcodedSymbol, MatchSymbol, Symbol, SymbolType,
    },
    resolve::{constants::NAMESPACE, util::ToMsgsResult},
    resolve::{
        function_arg::{FnArg, FnArgType},
        path::resolve_path,
        paths::{Crates, EnginePaths},
    },
    util::end,
};
use proc_macro2::{token_stream::IntoIter, TokenStream, TokenTree};
use quote::ToTokens;
use shared::util::{
    Call, Catch, FindFrom, Get, HandleErr, Increment, JoinMap, JoinMapInto, PushInto, SplitAround,
};

use shared::parse_args::{ComponentMacroArgs, GlobalMacroArgs, SystemMacroArgs};
use syn::{parenthesized, parse_macro_input, spanned::Spanned, Error, PatType, Token};

use super::{
    component_set::ComponentSet,
    parse_macro_call::{parse_macro_calls, update_macro_calls, ParseMacroCall},
    path::{ItemPath, ResolveResultTrait},
    paths::{ExpandEnum, GetPaths, MacroPaths, NamespaceTraits, Paths},
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
pub struct ItemSystem {
    pub path: ItemPath,
    pub args: Vec<FnArg>,
    pub attr_args: SystemMacroArgs,
}

#[derive(Debug)]
pub struct ItemsCrateDependency {
    pub cr_idx: usize,
    pub cr_alias: String,
}

#[derive(Debug)]
pub struct AstSymbolCounts {
    pub component_cnt: usize,
    pub global_cnt: usize,
    pub event_cnt: usize,
    pub component_set_cnt: usize,
}

// Pass 2: use resolving
// Resolve macro paths - convert to engine items
#[derive(Debug)]
pub struct ItemsCrate {
    pub dir: PathBuf,
    pub cr_name: String,
    pub cr_idx: usize,
    pub components: Vec<ItemComponent>,
    pub globals: Vec<ItemGlobal>,
    pub traits: Vec<ItemTrait>,
    pub events: Vec<ItemEvent>,
    pub systems: Vec<ItemSystem>,
    pub dependencies: Vec<ItemsCrateDependency>,
    // pub component_sets: Vec<component_set::ComponentSet>,
}

// Find items
impl ItemsCrate {
    pub fn find_component<'a>(&'a self, p: &ItemPath) -> Option<(usize, &'a ItemComponent)> {
        self.components
            .iter()
            .enumerate()
            .find_map(|(i, c)| (&c.path == p).then_some((i, c)))
    }

    pub fn find_global<'a>(&'a self, p: &ItemPath) -> Option<(usize, &'a ItemGlobal)> {
        self.globals
            .iter()
            .enumerate()
            .find_map(|(i, g)| (&g.path == p).then_some((i, g)))
    }

    pub fn find_trait<'a>(&'a self, p: &ItemPath) -> Option<(usize, &'a ItemTrait)> {
        self.traits
            .iter()
            .enumerate()
            .find_map(|(i, t)| (t.path.path == p.path).then_some((i, t)))
    }

    pub fn find_event<'a>(&'a self, p: &ItemPath) -> Option<(usize, &'a ItemEvent)> {
        self.events
            .iter()
            .enumerate()
            .find_map(|(i, e)| (&e.path == p).then_some((i, e)))
    }

    // pub fn find_component_set<'a>(
    //     &'a self,
    //     p: &ItemPath,
    // ) -> Option<(usize, &'a component_set::ComponentSet)> {
    //     self.component_sets
    //         .iter()
    //         .enumerate()
    //         .find_map(|(i, cs)| (&cs.path == p).then_some((i, cs)))
    // }
}

// Used to ignore a macros crate when iterating
fn iter_except<'a>(crates: &'a Vec<AstCrate>, cr_idx: usize) -> impl Iterator<Item = &'a AstCrate> {
    crates
        .iter()
        .enumerate()
        .filter(move |(i, _)| i != &cr_idx)
        .map(|(_, v)| v)
}

fn iter_except_mut<'a>(
    crates: &'a mut Vec<AstCrate>,
    cr_idx: usize,
) -> impl Iterator<Item = &'a mut AstCrate> {
    crates
        .iter_mut()
        .enumerate()
        .filter(move |(i, _)| i != &cr_idx)
        .map(|(_, v)| v)
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
}

// Parse AstCrate
impl ItemsCrate {
    fn new(cr: &AstCrate, paths: &Paths, crates: &Vec<AstCrate>) -> Self {
        Self {
            dir: cr.dir.to_owned(),
            cr_name: cr.name.to_string(),
            cr_idx: cr.idx,
            components: Vec::new(),
            globals: Vec::new(),
            traits: Vec::new(),
            events: Vec::new(),
            systems: Vec::new(),
            dependencies: cr
                .deps
                .iter()
                .map(|(&cr_idx, alias)| ItemsCrateDependency {
                    cr_idx,
                    cr_alias: alias.to_string(),
                })
                .collect(),
            // component_sets: Vec::new(),
        }
    }

    pub fn parse(paths: &Paths, crates: &mut Vec<AstCrate>) -> Vec<Self> {
        let macro_cr_idx = paths.get_cr_idx(Crates::Macros);

        let mut errs = Vec::new();
        let mut items = Items::new();

        // Insert hardcoded symbols
        for sym in HardcodedSymbol::VARIANTS {
            AstCrate::add_hardcoded_symbol(crates, paths, sym)
        }

        // Resolve components, globals, and events
        let symbols = iter_except(crates, macro_cr_idx).map_vec(|cr| {
            cr.iter_mods().map_vec(|m| {
                m.items
                    .structs
                    .iter()
                    .map(|s| (&s.path, &s.data.attrs))
                    .chain(m.items.enums.iter().map(|e| (&e.path, &e.data.attrs)))
                    .filter_map_vec(|(path, attrs)| {
                        // Search through attributes
                        attrs
                            .iter()
                            .find_map(|attr| {
                                resolve_path(attr.path.to_vec(), (m, cr, crates))
                                    .expect_symbol()
                                    .expect_any_hardcoded()
                                    .discard_symbol()
                                    .ok()
                                    .and_then(|hard_sym| match hard_sym {
                                        HardcodedSymbol::ComponentMacro => {
                                            items.components.push(ItemComponent {
                                                path: ItemPath {
                                                    cr_idx: cr.idx,
                                                    path: path.to_vec(),
                                                },
                                                args: ComponentMacroArgs::from(&attr.args),
                                            });
                                            Some(SymbolType::Component(items.components.len() - 1))
                                        }
                                        HardcodedSymbol::GlobalMacro => {
                                            items.globals.push(ItemGlobal {
                                                path: ItemPath {
                                                    cr_idx: cr.idx,
                                                    path: path.to_vec(),
                                                },
                                                args: GlobalMacroArgs::from(&attr.args),
                                            });
                                            Some(SymbolType::Global(items.globals.len() - 1))
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
        for (cr, crate_symbols) in iter_except_mut(crates, macro_cr_idx).zip(symbols) {
            let mut mods = cr.iter_mods_mut();
            for mod_symbols in crate_symbols {
                if let Some(m) = mods.next(cr) {
                    m.symbols.extend(mod_symbols)
                }
            }
        }

        // Resolve component sets
        let symbols = iter_except(crates, macro_cr_idx).map_vec(|cr| {
            cr.iter_mods().map_vec(|m| {
                m.items.macro_calls.filter_map_vec(|call| {
                    resolve_path(call.path.to_vec(), (m, cr, crates))
                        .expect_symbol()
                        .expect_hardcoded(HardcodedSymbol::ComponentsMacro)
                        .ok()
                        .and_then(|_| {
                            ComponentSet::parse(call.data.args.clone(), (m, cr, crates))
                                .handle_err(|es| {
                                    errs.push(String::new());
                                    errs.extend(es);
                                })
                                .map(|cs| {
                                    let path = cs.path.path.to_vec();
                                    items.component_sets.push(cs);
                                    Symbol {
                                        kind: SymbolType::ComponentSet(
                                            items.component_sets.len() - 1,
                                        ),
                                        path,
                                        public: true,
                                    }
                                })
                        })
                })
            })
        });

        // Insert component set symbols
        for (cr, crate_symbols) in iter_except_mut(crates, macro_cr_idx).zip(symbols) {
            let mut mods = cr.iter_mods_mut();
            for mod_symbols in crate_symbols {
                if let Some(m) = mods.next(cr) {
                    m.symbols.extend(mod_symbols);
                }
            }
        }

        // Insert namespace mod
        for cr in iter_except_mut(crates, macro_cr_idx) {
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

            let mut m = cr.add_mod(path.push_into(NAMESPACE.to_string()));
            m.uses = uses;
        }

        // Insert trait symbols
        for tr in NamespaceTraits::VARIANTS {
            let path = tr.full_path();
            let gl_path = tr.get_global().full_path();
            // Reference global in entry crate
            items.globals.push(ItemGlobal {
                path: ItemPath {
                    cr_idx: 0,
                    path: path.to_vec(),
                },
                args: GlobalMacroArgs::from(&Vec::new()),
            });
            for cr in iter_except_mut(crates, macro_cr_idx) {
                let idx = items.globals.len() - 1;
                // Add globals to entry crates
                if cr.idx == 0 {
                    cr.add_symbol(Symbol {
                        kind: SymbolType::Global(idx),
                        path: gl_path.to_vec(),
                        public: true,
                    })
                }
                // Add traits to all crates
                cr.add_symbol(Symbol {
                    kind: SymbolType::Trait(idx),
                    path: path.to_vec(),
                    public: true,
                })
            }
        }

        // Resolve systems
        let symbols = iter_except(crates, macro_cr_idx).map_vec(|cr| {
            cr.iter_mods().map_vec(|m| {
                m.items.functions.iter().filter_map_vec(|fun| {
                    fun.data.attrs.iter().find_map(|attr| {
                        resolve_path(attr.path.to_vec(), (m, cr, crates))
                            .expect_symbol()
                            .expect_hardcoded(HardcodedSymbol::SystemMacro)
                            .ok()
                            .and_then(|sym| {
                                let path =
                                    m.path.to_vec().push_into(fun.data.sig.ident.to_string());
                                let attr_args = SystemMacroArgs::from(&attr.args);
                                FnArg::parse(&attr_args, &items, &fun.data.sig, (m, cr, crates))
                                    .handle_err(|es| {
                                        errs.push(String::new());
                                        errs.extend(es);
                                    })
                                    .map(|args| {
                                        items.systems.push(ItemSystem {
                                            path: ItemPath {
                                                cr_idx: cr.idx,
                                                path: path.to_vec(),
                                            },
                                            args,
                                            attr_args,
                                        });
                                        Symbol {
                                            kind: SymbolType::System(items.systems.len() - 1),
                                            path,
                                            public: true,
                                        }
                                    })
                            })
                    })
                })
            })
        });

        // Insert system symbols
        for (cr, crate_symbols) in iter_except_mut(crates, macro_cr_idx).zip(symbols) {
            let mut mods = cr.iter_mods_mut();
            for mod_symbols in crate_symbols {
                if let Some(m) = mods.next(cr) {
                    m.symbols.extend(mod_symbols)
                }
            }
        }

        // Get paths from each crate to every other crate
        let paths = CratePaths::new(crates);
        eprintln!("{paths:#?}");

        // Generate globals struct code

        // Generate components struct code

        // Generate events struct/enum code

        // Generate component set code for building vectors

        // Generate system call code

        // Generate app struct

        if !errs.is_empty() {
            eprintln!("{}", errs.join("\n"));
            panic!("")
        }

        // Add namespace mod to all crates except macro
        // Must happen after the dependencies are set
        // let end = end(&crates, 1);
        // for cr in crates[..end].iter_mut() {
        //     cr.add_namespace_mod()
        // }

        // Skip macros crate

        // Pass 1: Add symbols
        // Step 1.1: Parse component set macro calls
        // let components_path = paths.get_macro(MacroPaths::Components);
        // let comp_sets = iter_except(crates, macro_cr_idx)
        //     .map(|cr| parse_macro_calls(components_path, &cr.main, cr, crates))
        //     .collect::<Vec<_>>();

        // // Step 1.2: Update mods with component sets
        // let component_sets = iter_except_mut(crates, macro_cr_idx)
        //     .zip(comp_sets)
        //     .map(|(cr, comp_sets)| update_macro_calls(comp_sets, &mut cr.main))
        //     .collect::<Vec<_>>();

        // Pass 2: Resolve all paths to cannonical forms
        //     Prereq: All symbols must be added
        // Step 2.1: Resolve components, globals, events, and systems
        // let mut items = iter_except(crates, macro_cr_idx)
        //     .map(|cr| {
        //         let mut item = ItemsCrate::new(cr, &paths, &crates);
        //         // Macros crate must be included as a dependency to resolve macro calls
        //         item.resolve_items(cr, &cr.main, &paths, &crates);
        //         // Remove macros crate as crate dependency
        //         item.remove_macros_crate(paths);
        //         item
        //     })
        //     .collect();

        // // Step 2.2: Add traits
        // Self::add_traits(&mut items);

        // // Pass 3: Validate all items
        // // Step 3.1: Validate component sets with components
        // //     Prereq: All components must be added
        // let component_sets = component_sets
        //     .map_vec(|cs_vec| cs_vec.map_vec(|cs| component_set::ComponentSet::parse(cs, &items)));
        // for (item, cs) in items.iter_mut().zip(component_sets) {
        //     item.component_sets = cs;
        // }

        // Step 3.2: Validate system args
        //     Prereq: All components, globals, events, and component sets must be added

        // items
        Vec::new()
    }

    fn remove_macros_crate(&mut self, paths: &Paths) {
        if let Some(i) = self
            .dependencies
            .iter()
            .position(|d| d.cr_idx == paths.get_cr_idx(Crates::Macros))
        {
            self.dependencies.swap_remove(i);
        }
    }
}
