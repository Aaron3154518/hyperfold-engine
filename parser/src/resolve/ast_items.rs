use std::path::PathBuf;

use crate::{
    codegen::mods::entry_namespace_items,
    parse::{
        ast_crate::Crate,
        ast_fn_arg::{FnArg, FnArgType},
        ast_mod::{MarkType, Mod},
    },
    resolve::ast_resolve::resolve_path,
    util::{Catch, Get},
};

use shared::parse_args::{ComponentMacroArgs, GlobalMacroArgs, SystemMacroArgs};

use super::{
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
    pub cr_idx: usize,
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
    pub events: Vec<Event>,
    pub systems: Vec<System>,
    pub dependencies: Vec<Dependency>,
}

impl ItemsCrate {
    pub fn new() -> Self {
        Self {
            dir: PathBuf::new(),
            cr_name: String::new(),
            cr_idx: 0,
            components: Vec::new(),
            globals: Vec::new(),
            events: Vec::new(),
            systems: Vec::new(),
            dependencies: Vec::new(),
        }
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
        self.parse_mod(cr, &cr.main, paths, crates);

        // Add globals to entry crate
        if self.cr_idx == 0 {
            entry_namespace_items(self);
        }
    }

    pub fn parse_mod(&mut self, cr: &Crate, m: &Mod, paths: &Paths, crates: &Vec<Crate>) {
        let cr_idx = cr.idx;

        for mi in m.marked.iter() {
            for (path, args) in mi.attrs.iter() {
                let match_path = resolve_path(path.to_vec(), cr, m, crates).get();
                match &mi.ty {
                    MarkType::Struct => {
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
