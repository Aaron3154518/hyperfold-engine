use std::{collections::VecDeque, env::temp_dir, fs, path::PathBuf};

use crate::{
    codegen2::{Codegen, Crates},
    // codegen::component_set::{self},
    parse::{
        AstCrate, AstMod, AstUse, DiscardSymbol, HardcodedSymbol, MatchSymbol, Symbol, SymbolType,
    },
    resolve::{
        constants::{INDEX, INDEX_SEP, NAMESPACE},
        util::ToMsgsResult,
    },
    resolve::{
        function_arg::{FnArg, FnArgType},
        path::resolve_path,
        paths::{Crate, EnginePaths},
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
    paths::{ExpandEnum, GetPaths, MacroPaths, NamespaceTraits},
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
    fn new(cr: &AstCrate, crates: &Vec<AstCrate>) -> Self {
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

    pub fn parse(crates: &mut Crates) -> Vec<Self> {
        let mut errs = Vec::new();
        let mut items = Items::new();

        // Insert hardcoded symbols
        for sym in HardcodedSymbol::VARIANTS {
            AstCrate::add_hardcoded_symbol(crates, sym)
        }

        // Resolve components, globals, and events
        let symbols = crates.iter().map_vec(|cr| {
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
                                resolve_path(attr.path.to_vec(), (m, cr, crates.get_crates()))
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
        for (cr, crate_symbols) in crates.iter_mut().zip(symbols) {
            let mut mods = cr.iter_mods_mut();
            for mod_symbols in crate_symbols {
                if let Some(m) = mods.next(cr) {
                    m.symbols.extend(mod_symbols)
                }
            }
        }

        // Resolve component sets
        let symbols = crates.iter().map_vec(|cr| {
            cr.iter_mods().map_vec(|m| {
                m.items.macro_calls.filter_map_vec(|call| {
                    resolve_path(call.path.to_vec(), (m, cr, crates.get_crates()))
                        .expect_symbol()
                        .expect_hardcoded(HardcodedSymbol::ComponentsMacro)
                        .ok()
                        .and_then(|_| {
                            ComponentSet::parse(
                                call.data.args.clone(),
                                (m, cr, crates.get_crates()),
                            )
                            .handle_err(|es| {
                                errs.push(String::new());
                                errs.extend(es);
                            })
                            .map(|cs| {
                                let path = cs.path.path.to_vec();
                                items.component_sets.push(cs);
                                Symbol {
                                    kind: SymbolType::ComponentSet(items.component_sets.len() - 1),
                                    path,
                                    public: true,
                                }
                            })
                        })
                })
            })
        });

        // Insert component set symbols
        for (cr, crate_symbols) in crates.iter_mut().zip(symbols) {
            let mut mods = cr.iter_mods_mut();
            for mod_symbols in crate_symbols {
                if let Some(m) = mods.next(cr) {
                    m.symbols.extend(mod_symbols);
                }
            }
        }

        // Insert namespace mod
        for cr in crates.iter_mut() {
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
            for cr in crates.iter_mut() {
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
        let symbols = crates.iter().map_vec(|cr| {
            cr.iter_mods().map_vec(|m| {
                m.items.functions.iter().filter_map_vec(|fun| {
                    fun.data.attrs.iter().find_map(|attr| {
                        resolve_path(attr.path.to_vec(), (m, cr, crates.get_crates()))
                            .expect_symbol()
                            .expect_hardcoded(HardcodedSymbol::SystemMacro)
                            .ok()
                            .and_then(|sym| {
                                let path =
                                    m.path.to_vec().push_into(fun.data.sig.ident.to_string());
                                let attr_args = SystemMacroArgs::from(&attr.args);
                                FnArg::parse(
                                    &attr_args,
                                    &items,
                                    &fun.data.sig,
                                    (m, cr, crates.get_crates()),
                                )
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
        for (cr, crate_symbols) in crates.iter_mut().zip(symbols) {
            let mut mods = cr.iter_mods_mut();
            for mod_symbols in crate_symbols {
                if let Some(m) = mods.next(cr) {
                    m.symbols.extend(mod_symbols)
                }
            }
        }

        // Generate globals struct code
        let globs = Codegen::globals(0, &items.globals, &crates);

        // Generate components struct code

        // Generate events struct/enum code

        // Generate component set code for building vectors

        // Generate system call code

        // Generate app struct

        // Write codegen to file
        match globs {
            Ok(globs) => {
                let code = vec![globs.to_string(), String::new()];

                let out =
                    PathBuf::from(std::env::var("OUT_DIR").expect("No out directory specified"));

                // Create index file
                fs::write(
                    temp_dir().join(INDEX),
                    crates
                        .iter()
                        .zip(code)
                        .enumerate()
                        .map_vec(|(i, (cr, code))| {
                            // Write to file
                            let file = out.join(format!("{}.rs", i));
                            fs::write(file.to_owned(), code)
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
                .catch(format!("Could not write to index file: {INDEX}"))
            }
            Err(err) => errs.extend(err),
        }

        if !errs.is_empty() {
            eprintln!("{}", errs.join("\n"));
            panic!("")
        }

        Vec::new()
    }
}
