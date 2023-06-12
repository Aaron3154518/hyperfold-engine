use std::{collections::VecDeque, path::PathBuf};

use crate::{
    codegen::mods::add_traits,
    parse::{
        ast_crate::Crate,
        ast_fn_arg::{FnArg, FnArgType},
        ast_mod::{MarkType, Mod},
    },
    resolve::ast_resolve::resolve_path,
    util::end,
};
use proc_macro2::{token_stream::IntoIter, TokenStream, TokenTree};
use quote::ToTokens;
use shared::util::{Call, Catch, FindFrom, Get, JoinMap, SplitAround};

use shared::parse_args::{ComponentMacroArgs, GlobalMacroArgs, SystemMacroArgs};
use syn::{parenthesized, parse_macro_input, spanned::Spanned, Error, Token};

use super::{
    ast_component_set::ComponentSet,
    ast_paths::{MacroPaths, Paths},
    ast_resolve::Path,
};

#[derive(Debug)]
pub struct Component {
    pub path: Path,
    pub args: ComponentMacroArgs,
}

#[derive(Clone, Debug)]
pub struct Global {
    pub path: Path,
    pub args: GlobalMacroArgs,
}

#[derive(Clone, Debug)]
pub struct Trait {
    pub path: Path,
    pub g_idx: usize,
}

#[derive(Debug)]
pub struct Event {
    pub path: Path,
}

#[derive(Debug)]
pub struct System {
    pub path: Path,
    pub args: Vec<FnArg>,
    pub attr_args: SystemMacroArgs,
}

#[derive(Debug)]
pub struct Dependency {
    pub cr_idx: usize,
    pub cr_alias: String,
}

// Pass 2: use resolving
// Resolve macro paths - convert to engine items
#[derive(Debug)]
pub struct ItemsCrate {
    pub dir: PathBuf,
    pub cr_name: String,
    pub cr_idx: usize,
    pub components: Vec<Component>,
    pub globals: Vec<Global>,
    pub traits: Vec<Trait>,
    pub events: Vec<Event>,
    pub systems: Vec<System>,
    pub dependencies: Vec<Dependency>,
    pub component_sets: Vec<ComponentSet>,
}

pub trait ParseMacroCall
where
    Self: Sized,
{
    fn parse(cr: &Crate, m: &Mod, ts: TokenStream) -> syn::Result<Self>;

    fn update_mod(&self, m: &mut Mod);
}

struct MacroCalls<T> {
    calls: Vec<T>,
    mods: Vec<MacroCalls<T>>,
}

impl ItemsCrate {
    pub fn new() -> Self {
        Self {
            dir: PathBuf::new(),
            cr_name: String::new(),
            cr_idx: 0,
            components: Vec::new(),
            globals: Vec::new(),
            traits: Vec::new(),
            events: Vec::new(),
            systems: Vec::new(),
            dependencies: Vec::new(),
            component_sets: Vec::new(),
        }
    }

    fn parse_macro_calls<T>(
        macro_path: &Path,
        m: &Mod,
        cr: &Crate,
        crates: &Vec<Crate>,
    ) -> MacroCalls<T>
    where
        T: ParseMacroCall,
    {
        MacroCalls {
            calls: m
                .macro_calls
                .iter()
                .filter_map(|mc| {
                    (&resolve_path(mc.path.to_vec(), cr, m, crates).get() == macro_path)
                        .then_some(())
                        .and_then(|_| T::parse(cr, m, mc.args.clone()).ok())
                })
                .collect(),
            mods: m
                .mods
                .map_vec(|m| Self::parse_macro_calls(macro_path, m, cr, crates)),
        }
    }

    fn update_macro_calls<T>(macro_calls: MacroCalls<T>, m: &mut Mod) -> Vec<T>
    where
        T: ParseMacroCall,
    {
        let mut calls = macro_calls.calls;
        calls.iter().for_each(|c| c.update_mod(m));
        for (macro_calls, m) in macro_calls.mods.into_iter().zip(m.mods.iter_mut()) {
            calls.extend(Self::update_macro_calls(macro_calls, m));
        }
        calls
    }

    pub fn parse(paths: &Paths, crates: &mut Vec<Crate>) -> Vec<Self> {
        // Skip macros crate
        let last = end(&crates, 1);
        let parse_crates = &crates[..last];
        let mut items = parse_crates.map_vec(|_| ItemsCrate::new());

        // Pass 1: Parse component set macro calls
        let components_path = paths.get_macro(MacroPaths::Components);
        let comp_sets = parse_crates
            .map_vec(|cr| Self::parse_macro_calls(components_path, &cr.main, cr, crates));

        let mut parse_crates = &mut crates[..last];

        // Pass 2: Update mods with component sets
        for ((item, comp_sets), cr) in items
            .iter_mut()
            .zip(comp_sets.into_iter())
            .zip(parse_crates)
        {
            item.component_sets = Self::update_macro_calls(comp_sets, &mut cr.main);
        }

        let parse_crates = &crates[..last];

        for (item, cr) in items.iter_mut().zip(parse_crates) {
            item.parse_crate(cr, &paths, &crates);
            // Remove macros crate as crate dependency
            if let Some(i) = item
                .dependencies
                .iter()
                .position(|d| d.cr_idx == crates.len() - 1)
            {
                item.dependencies.swap_remove(i);
            }
        }
        add_traits(&mut items);
        items
    }

    pub fn parse_crate(&mut self, cr: &Crate, paths: &Paths, crates: &Vec<Crate>) {
        self.dir = cr.dir.to_owned();
        self.cr_name = cr.name.to_string();
        self.cr_idx = cr.idx;
        self.dependencies = cr
            .deps
            .iter()
            .map(|(&cr_idx, alias)| Dependency {
                cr_idx,
                cr_alias: alias.to_string(),
            })
            .collect::<Vec<_>>();
        self.parse_mod(cr, &cr.main, paths, crates)
    }

    pub fn parse_mod(&mut self, cr: &Crate, m: &Mod, paths: &Paths, crates: &Vec<Crate>) {
        let cr_idx = cr.idx;

        for mi in m.marked.iter() {
            for (path, args) in mi.attrs.iter() {
                let match_path = resolve_path(path.to_vec(), cr, m, crates).get();
                match &mi.ty {
                    MarkType::Enum | MarkType::Struct => {
                        if &match_path == paths.get_macro(MacroPaths::Component) {
                            self.components.push(Component {
                                path: Path {
                                    cr_idx,
                                    path: mi.sym.path.to_vec(),
                                },
                                args: ComponentMacroArgs::from(args.to_vec()),
                            });
                            break;
                        } else if &match_path == paths.get_macro(MacroPaths::Global) {
                            self.globals.push(Global {
                                path: Path {
                                    cr_idx,
                                    path: mi.sym.path.to_vec(),
                                },
                                args: GlobalMacroArgs::from(args.to_vec()),
                            });
                            break;
                        } else if &match_path == paths.get_macro(MacroPaths::Event) {
                            self.events.push(Event {
                                path: Path {
                                    cr_idx,
                                    path: mi.sym.path.to_vec(),
                                },
                            })
                        }
                    }
                    MarkType::Fn { args: fn_args } => {
                        if &match_path == paths.get_macro(MacroPaths::System) {
                            self.systems.push(System {
                                path: Path {
                                    cr_idx,
                                    path: mi.sym.path.to_vec(),
                                },
                                args: fn_args
                                    .iter()
                                    .map(|a| {
                                        let mut a = a.to_owned();
                                        a.resolve_paths(cr, m, crates);
                                        a
                                    })
                                    .collect(),
                                attr_args: SystemMacroArgs::from(args.to_vec()),
                            });
                            break;
                        }
                    }
                }
            }
        }

        m.mods
            .iter()
            .for_each(|m| self.parse_mod(cr, m, paths, crates));
    }
}
