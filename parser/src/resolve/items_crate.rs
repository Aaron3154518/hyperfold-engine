use quote::quote;
use std::{collections::VecDeque, env::temp_dir, fs, path::PathBuf};

use crate::{
    codegen::{self as codegen, Crates, Traits},
    match_ok,
    parse::{
        AstCrate, AstMod, AstUse, ComponentSymbol, DiscardSymbol, GlobalSymbol, HardcodedSymbol,
        MatchSymbol, Symbol, SymbolType,
    },
    resolve::{
        constants::{INDEX, INDEX_SEP, NAMESPACE},
        paths::{NAMESPACE_USE_STMTS, TRAITS},
        util::{CombineMsgs, Zip2Msgs, Zip4Msgs, Zip5Msgs, Zip6Msgs, Zip7Msgs, Zip8Msgs, Zip9Msgs},
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
use syn::{
    parenthesized, parse_macro_input, spanned::Spanned, token::Trait, Error, PatType, Token,
};

use super::{
    component_set::ComponentSet,
    parse_macro_call::{parse_macro_calls, update_macro_calls, ParseMacroCall},
    path::{ItemPath, ResolveResultTrait},
    paths::ExpandEnum,
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

        let main_cr_idx = crates.get_crate_index(Crate::Main);
        let macro_cr_idx = crates.get_crate_index(Crate::Macros);

        // Insert hardcoded symbols
        for sym in HardcodedSymbol::VARIANTS {
            AstCrate::add_hardcoded_symbol(crates, sym)
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
                                    .expect_symbol()
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

            let mut m = cr.add_mod(path.push_into(NAMESPACE.to_string()));
            m.uses = uses;
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
                }
                // Add traits to all crates
                cr.add_symbol(Symbol {
                    kind: SymbolType::Trait(g_sym),
                    path: path.to_vec(),
                    public: true,
                })
            }
        }

        // Resolve systems
        let symbols = crates.iter_except([macro_cr_idx]).map_vec_into(|cr| {
            cr.iter_mods().map_vec_into(|m| {
                m.items.functions.iter().filter_map_vec_into(|fun| {
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
        for (cr, crate_symbols) in crates.iter_except_mut([macro_cr_idx]).zip(symbols) {
            let mut mods = cr.iter_mods_mut();
            for mod_symbols in crate_symbols {
                if let Some(m) = mods.next(cr) {
                    m.symbols.extend(mod_symbols)
                }
            }
        }

        // Generate globals struct
        let globals = codegen::globals(main_cr_idx, &items.globals, crates);

        // Generate components struct
        let components = codegen::components(main_cr_idx, &items.components, crates);

        // Generate component trait implementations
        let component_traits =
            codegen::component_trait_impls(main_cr_idx, &items.components, crates);

        // Generate events enum
        let events_enum = codegen::events_enum(&items.events);

        // Generate events struct
        let events = codegen::events(main_cr_idx, &items.events, crates);

        // Generate event trait implementations
        let event_traits = codegen::event_trait_impls(main_cr_idx, &items.events, crates);

        // Generate event/component traits
        let trait_defs = crates
            .iter_except([macro_cr_idx])
            .map_vec_into(|cr| {
                let add_event = codegen::event_trait_defs(cr.idx, &items.events, crates);
                let add_component =
                    codegen::component_trait_defs(cr.idx, &items.components, crates);
                match_ok!(Zip2Msgs, add_event, add_component, {
                    Traits {
                        add_event,
                        add_component,
                    }
                })
            })
            .combine_msgs();

        // Generate manager struct
        let manager_def = codegen::manager_def();
        let manager_impl = codegen::manager_impl(main_cr_idx, &items, crates);

        // Generate use statements for namespace
        let use_stmts = crates
            .iter_except([macro_cr_idx])
            .map_vec_into(|cr| {
                NAMESPACE_USE_STMTS
                    .map_vec(|path| crates.get_syn_path(cr.idx, path))
                    .combine_msgs()
            })
            .combine_msgs()
            .map(|use_stmts| use_stmts.map_vec_into(|stmts| quote!(#(pub use #stmts;)*)));

        // Write codegen to file
        match_ok!(
            Zip8Msgs,
            globals,
            components,
            component_traits,
            events,
            event_traits,
            trait_defs,
            manager_impl,
            use_stmts,
            {
                write_codegen(CodegenArgs {
                    crates,
                    globals,
                    components,
                    component_traits,
                    events_enum,
                    events,
                    event_traits,
                    trait_defs,
                    manager_def,
                    manager_impl,
                    use_stmts,
                })
            },
            err,
            { errs.extend(err) }
        );

        if !errs.is_empty() {
            eprintln!("{}", errs.join("\n"));
            panic!("")
        }

        Vec::new()
    }
}

struct CodegenArgs<'a> {
    crates: &'a Crates,
    globals: TokenStream,
    components: TokenStream,
    component_traits: TokenStream,
    events_enum: TokenStream,
    events: TokenStream,
    event_traits: TokenStream,
    trait_defs: Vec<Traits>,
    manager_def: TokenStream,
    manager_impl: TokenStream,
    use_stmts: Vec<TokenStream>,
}

fn write_codegen<'a>(
    CodegenArgs {
        crates,
        globals,
        components,
        component_traits,
        events_enum,
        events,
        event_traits,
        trait_defs,
        manager_def,
        manager_impl,
        use_stmts,
    }: CodegenArgs<'a>,
) {
    let main_cr_idx = crates.get_crate_index(Crate::Main);
    let mut code = trait_defs
        .into_iter()
        .zip(use_stmts)
        .enumerate()
        .map_vec_into(
            |(
                cr_idx,
                (
                    Traits {
                        add_event,
                        add_component,
                    },
                    use_stmts,
                ),
            )| {
                // TODO: no newlines
                if cr_idx == main_cr_idx {
                    format!(
                        "pub mod {NAMESPACE} {{\
                            {use_stmts}\
                            {globals}\
                            {components}\
                            {add_component}\
                            {component_traits}\
                            {events_enum}\
                            {events}\
                            {add_event}\
                            {event_traits}\
                            {manager_def}\
                            {manager_impl}\
                        }}"
                    )
                } else {
                    format!(
                        "pub mod {NAMESPACE} {{\
                            {use_stmts}\
                            {add_component}\
                            {add_event}\
                        }}"
                    )
                }
            },
        );

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("No out directory specified"));

    // Create index file
    fs::write(
        temp_dir().join(INDEX),
        crates
            .iter()
            .zip(code)
            .enumerate()
            .map_vec_into(|(i, (cr, code))| {
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
